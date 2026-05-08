import re
path = 'src/persistence_migrations.rs'
with open(path, 'r') as f:
    content = f.read()

v9_migration = """            if version == 9
                && !self.column_exists(conn, "csm_concepts", "vector_format").await?
            {
                conn.execute_batch(
                    "ALTER TABLE csm_concepts ADD COLUMN vector_format TEXT NOT NULL DEFAULT 'f32';",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v9: {e}")))?;
            }

            if version == 9
                && !self.column_exists(conn, "csm_versions", "vector_format").await?
            {
                conn.execute_batch(
                    "ALTER TABLE csm_versions ADD COLUMN vector_format TEXT NOT NULL DEFAULT 'f32';",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v9 versions: {e}")))?;
            }"""

content = re.sub(r'if version == 9.*?csm_schema_version\(version\)', v9_migration + "\n\n            conn.execute(\n                \"INSERT INTO csm_schema_version(version) VALUES (?1)\",", content, flags=re.DOTALL)

with open(path, 'w') as f:
    f.write(content)
