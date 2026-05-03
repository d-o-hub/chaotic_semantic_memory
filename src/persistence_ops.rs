use libsql::params;
use tokio::fs;
use tracing::{info, warn};

use crate::error::{MemoryError, Result};
use crate::persistence::{ConceptVersion, Persistence};

impl Persistence {
    pub async fn save_associations(&self, associations: &[(String, String, f32)]) -> Result<()> {
        if associations.is_empty() {
            return Ok(());
        }

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        let stmt = conn
            .prepare(
                "INSERT OR REPLACE INTO csm_associations (from_id, to_id, strength)
                 VALUES (?1, ?2, ?3)",
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to prepare statement: {e}")))?;

        let mut first_error: Option<MemoryError> = None;
        for (from, to, strength) in associations {
            stmt.reset();
            if let Err(e) = stmt
                .execute(params![from.clone(), to.clone(), *strength])
                .await
            {
                first_error = Some(MemoryError::database(format!(
                    "Failed to batch save association: {e}"
                )));
                break;
            }
        }

        if let Some(error) = first_error {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(error);
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {e}")))?;

        Ok(())
    }

    pub async fn clear_all(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute_batch(
            "BEGIN;
             DELETE FROM csm_associations;
             DELETE FROM csm_versions;
             DELETE FROM csm_concepts;
             COMMIT;",
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear all data: {e}")))?;
        Ok(())
    }

    pub async fn get_concept_history(&self, id: &str, limit: usize) -> Result<Vec<ConceptVersion>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT concept_id, version, vector, metadata, modified_at
                 FROM csm_versions
                 WHERE concept_id = ?1
                 ORDER BY version DESC
                 LIMIT ?2",
                libsql::params![id, limit as i64],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concept history: {e}")))?;

        let mut history = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch concept history row: {e}"))
        })? {
            let concept_id: String = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get concept_id: {e}")))?;
            let version: i64 = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get version: {e}")))?;
            let vector_bytes: Vec<u8> = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {e}")))?;
            let metadata_json: String = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {e}")))?;
            let modified_at: i64 = row
                .get(4)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {e}")))?;

            history.push(ConceptVersion {
                concept_id,
                version,
                vector: crate::hyperdim::HVec10240::from_bytes(&vector_bytes)?,
                metadata: serde_json::from_str(&metadata_json)?,
                modified_at: modified_at as u64,
            });
        }

        Ok(history)
    }

    pub async fn schema_version(&self) -> Result<i64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM csm_schema_version",
                (),
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to get schema version: {e}")))?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch schema version row: {e}"))
        })? {
            let version: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse schema version: {e}"))
            })?;
            Ok(version)
        } else {
            Ok(0)
        }
    }

    pub async fn apply_migrations(&self, target_version: i64) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        self.apply_migrations_with_conn(&conn, target_version).await
    }

    pub async fn backup(&self, path: &str) -> Result<()> {
        let Some(_local_path) = &self.local_path else {
            return Err(MemoryError::UnsupportedOperation(
                "backup is only supported for local SQLite databases".to_string(),
            ));
        };

        self.checkpoint().await?;
        if fs::metadata(path).await.is_ok() {
            fs::remove_file(path).await.map_err(MemoryError::Io)?;
        }

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("VACUUM INTO ?1", params![path])
            .await
            .map_err(|e| MemoryError::database(format!("Failed to create backup: {e}")))?;
        Ok(())
    }

    pub async fn restore(&self, path: &str) -> Result<()> {
        let Some(_local_path) = &self.local_path else {
            return Err(MemoryError::UnsupportedOperation(
                "restore is only supported for local SQLite databases".to_string(),
            ));
        };

        fs::metadata(path).await.map_err(MemoryError::Io)?;

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN IMMEDIATE", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to begin restore transaction: {e}"))
        })?;

        if let Err(error) = async {
            conn.execute("ATTACH DATABASE ?1 AS restore_db", params![path])
                .await
                .map_err(|e| MemoryError::database(format!("Failed to attach backup DB: {e}")))?;

            conn.execute_batch(
                "DELETE FROM csm_associations;
                 DELETE FROM csm_versions;
                 DELETE FROM csm_concepts;
                 DELETE FROM csm_schema_version;",
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to clear current database: {e}")))?;

            conn.execute_batch(
                "INSERT INTO csm_concepts (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                 SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json FROM restore_db.csm_concepts;
                 INSERT INTO csm_associations (from_id, to_id, strength)
                 SELECT from_id, to_id, strength FROM restore_db.csm_associations;
                 INSERT INTO csm_versions (concept_id, version, vector, metadata, modified_at)
                 SELECT concept_id, version, vector, metadata, modified_at FROM restore_db.csm_versions;
                 INSERT INTO csm_schema_version(version)
                 SELECT version FROM restore_db.csm_schema_version;",
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to import backup data: {e}")))?;

            Ok::<(), MemoryError>(())
        }
        .await
        {
            let _ = conn.execute_batch("ROLLBACK;").await;
            return Err(error);
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit restore: {e}")))?;
        if let Err(error) = conn.execute_batch("DETACH DATABASE restore_db;").await {
            warn!(error = %error, "failed to detach restore_db after restore");
        }

        self.init_schema().await?;
        Ok(())
    }

    pub async fn health_check(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.query("SELECT 1", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed persistence health check: {e}")))?;
        Ok(())
    }

    /// Delete a single association between two concepts.
    pub async fn delete_association(&self, from: &str, to: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE from_id = ?1 AND to_id = ?2",
            params![from, to],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to delete association: {e}")))?;
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    pub async fn clear_concept_associations(&self, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE from_id = ?1",
            params![id],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear concept associations: {e}")))?;
        Ok(())
    }

    /// Internal migration method that reuses an existing connection.
    /// Used by init_schema() to avoid semaphore deadlock from nested permit acquisition.
    pub(crate) async fn apply_migrations_with_conn(
        &self,
        conn: &libsql::Connection,
        target_version: i64,
    ) -> Result<()> {
        let current = self.schema_version_with_conn(conn).await?;
        if target_version <= current {
            return Ok(());
        }

        conn.execute("BEGIN", ()).await.map_err(|e| {
            MemoryError::database(format!("Failed to begin migration transaction: {e}"))
        })?;

        for version in (current + 1)..=target_version {
            info!(version, "applying schema migration");
            if version == 2 {
                conn.execute_batch(
                    "CREATE INDEX IF NOT EXISTS idx_csm_versions_modified_at
                     ON csm_versions(modified_at);",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v2: {e}")))?;
            }

            if version == 3 {
                // Add expires_at column for TTL support
                if !self
                    .column_exists(conn, "csm_concepts", "expires_at")
                    .await?
                {
                    conn.execute_batch("ALTER TABLE csm_concepts ADD COLUMN expires_at INTEGER;")
                        .await
                        .map_err(|e| MemoryError::database(format!("Failed migration v3: {e}")))?;
                }
            }

            if version == 4 {
                // Add canonical_concepts table for semantic bridge
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS csm_canonical (
                        id TEXT PRIMARY KEY,
                        version INTEGER NOT NULL,
                        labels_json TEXT NOT NULL,
                        related_json TEXT NOT NULL
                    );",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v4: {e}")))?;
            }

            if version == 5 {
                self.apply_v5_namespace_migration(conn).await?;
            }

            if version == 6 {
                // Preserve semantic-bridge linkage IDs on concepts
                if !self
                    .column_exists(conn, "csm_concepts", "canonical_concept_ids_json")
                    .await?
                {
                    conn.execute_batch(
                        "ALTER TABLE csm_concepts ADD COLUMN canonical_concept_ids_json TEXT;",
                    )
                    .await
                    .map_err(|e| MemoryError::database(format!("Failed migration v6: {e}")))?;
                }
            }

            conn.execute(
                "INSERT INTO csm_schema_version(version) VALUES (?1)",
                libsql::params![version],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to record schema version: {e}")))?;
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit migrations: {e}")))?;

        Ok(())
    }

    /// Internal schema version query that reuses an existing connection.
    async fn schema_version_with_conn(&self, conn: &libsql::Connection) -> Result<i64> {
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM csm_schema_version",
                (),
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to get schema version: {e}")))?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch schema version row: {e}"))
        })? {
            let version: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse schema version: {e}"))
            })?;
            Ok(version)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
#[cfg(feature = "persistence")]
mod tests {
    use crate::hyperdim::HVec10240;
    use crate::persistence::Persistence;
    use crate::singularity::Concept;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn make_concept(id: &str) -> Concept {
        Concept {
            id: id.to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }

    #[tokio::test]
    async fn save_and_load_concept_roundtrip() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let concept = make_concept("test-concept");

        persistence
            .save_concept(&concept)
            .await
            .expect("Failed to save");
        let loaded = persistence
            .load_concept("test-concept")
            .await
            .expect("Failed to load")
            .expect("Concept not found");
        assert_eq!(loaded.id, concept.id);
    }

    #[tokio::test]
    async fn delete_concept_removes_from_db() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let concept = make_concept("delete-test");

        persistence
            .save_concept(&concept)
            .await
            .expect("Failed to save");
        persistence
            .delete_concept("delete-test")
            .await
            .expect("Failed to delete");
        let result = persistence.load_concept("delete-test").await;
        assert!(result.expect("Query failed").is_none());
    }

    #[tokio::test]
    async fn schema_version_initialized() {
        let temp = NamedTempFile::new().expect("Failed to create temp file");
        let path = temp.path().to_str().expect("Invalid path");
        let persistence = Persistence::new_local(path)
            .await
            .expect("Failed to create persistence");

        let version = persistence
            .schema_version()
            .await
            .expect("Failed to get version");
        assert!(version > 0);
    }
}
