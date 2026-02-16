---
name: swarm-advanced-features
description: "Export/import, versioning, migrations, and backup/restore. Use when adding enterprise/production features."
---

# Swarm: Advanced Features

## Workflow
1. Design format specification (JSON schema or binary protocol)
2. Implement streaming for large datasets
3. Add migration runner with version tracking
4. Implement backup/restore with integrity checks
5. Add tests for all operations
6. Document usage and recovery procedures

## Export/Import

```rust
pub async fn export_json(&self, path: &Path) -> Result<usize> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut count = 0;
    
    // Stream concepts in chunks
    for chunk in self.singularity.concepts().chunks(1000) {
        for concept in chunk {
            serde_json::to_writer(&mut writer, concept)?;
            count += 1;
        }
    }
    
    Ok(count)
}
```

## Versioning Schema

```sql
CREATE TABLE concept_versions (
    concept_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    vector BLOB NOT NULL,
    metadata TEXT NOT NULL,
    modified_at INTEGER NOT NULL,
    PRIMARY KEY (concept_id, version),
    FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
```

## Migration Runner

```rust
pub struct Migration {
    version: u32,
    up: &'static str,
    down: &'static str,
}

pub async fn run_migrations(&self, target: u32) -> Result<()> {
    let current = self.get_schema_version().await?;
    
    for migration in &MIGRATIONS[current as usize..target as usize] {
        let conn = self.connect().await?;
        conn.execute(migration.up, ()).await?;
        self.set_schema_version(migration.version).await?;
    }
    
    Ok(())
}
```

## Backup/Restore

```rust
pub async fn backup(&self, path: &Path) -> Result<()> {
    // SQLite: VACUUM INTO
    let conn = self.connect().await?;
    conn.execute(&format!("VACUUM INTO '{}'", path.display()), ()).await?;
    self.verify_backup(path).await?;
    Ok(())
}
```

## Format Specification

```json
{
  "version": "0.1.0",
  "exported_at": 1700000000,
  "schema_version": 5,
  "concepts": [...],
  "associations": [...],
  "concept_versions": [...]
}
```

## LOC Constraint
All files must remain ≤ 500 lines. Refactor to new modules if needed.
