use libsql::params;
use tokio::fs;
use tracing::info;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
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
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {}", e)))?;

        for (from, to, strength) in associations {
            conn.execute(
                "REPLACE INTO csm_associations (from_id, to_id, strength) VALUES (?1, ?2, ?3)",
                params![from.clone(), to.clone(), *strength],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to execute statement: {}", e)))?;
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    pub async fn delete_association(&self, from: &str, to: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE from_id = ?1 AND to_id = ?2",
            params![from, to],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to delete association: {}", e)))?;
        Ok(())
    }

    pub async fn clear_concept_associations(&self, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE from_id = ?1",
            params![id],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear associations: {}", e)))?;
        Ok(())
    }

    pub async fn clear_all(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute_batch(
            "BEGIN;
             DELETE FROM csm_associations;
             DELETE FROM csm_concepts;
             DELETE FROM csm_versions;
             COMMIT;",
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear all data: {}", e)))?;
        Ok(())
    }

    pub async fn backup(&self, path: &str) -> Result<()> {
        if self.local_path.is_none() {
            return Err(MemoryError::database(
                "Backup is only supported for local databases".to_string(),
            ));
        }

        fs::copy(self.local_path.as_ref().unwrap(), path)
            .await
            .map_err(|e| MemoryError::database(format!("Failed to copy database file: {}", e)))?;

        Ok(())
    }

    pub async fn restore(&self, path: &str) -> Result<()> {
        if self.local_path.is_none() {
            return Err(MemoryError::database(
                "Restore is only supported for local databases".to_string(),
            ));
        }

        fs::copy(path, self.local_path.as_ref().unwrap())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to copy database file: {}", e)))?;

        Ok(())
    }

    pub async fn get_concept_history(&self, id: &str, limit: usize) -> Result<Vec<ConceptVersion>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT version, vector, metadata, modified_at
                 FROM csm_versions WHERE concept_id = ?1
                 ORDER BY version DESC LIMIT ?2",
                params![id, limit as i64],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concept history: {}", e)))?;

        let mut versions = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {}", e)))?
        {
            let version: i64 = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get version: {}", e)))?;
            let vector_bytes: Vec<u8> = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {}", e)))?;
            let metadata_json: String = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {}", e)))?;
            let modified_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {}", e)))?;

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;

            versions.push(ConceptVersion {
                concept_id: id.to_string(),
                version,
                vector,
                metadata,
                modified_at: modified_at as u64,
            });
        }

        Ok(versions)
    }

    pub async fn schema_version(&self) -> Result<i64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        self.schema_version_with_conn(&conn).await
    }

    pub async fn apply_migrations(&self, target_version: i64) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        self.apply_migrations_with_conn(&conn, target_version).await
    }

    /// Internal migration logic that reuses an existing connection.
    pub(crate) async fn apply_migrations_with_conn(
        &self,
        conn: &libsql::Connection,
        target_version: i64,
    ) -> Result<()> {
        let mut current_version = self.schema_version_with_conn(conn).await?;

        while current_version < target_version {
            info!(
                "Applying migration from version {} to {}",
                current_version,
                current_version + 1
            );
            match current_version {
                1 => {
                    conn.execute_batch(
                        "BEGIN;
                         CREATE TABLE IF NOT EXISTS __schema_version (version INTEGER PRIMARY KEY);
                         INSERT OR IGNORE INTO __schema_version (version) VALUES (1);
                         COMMIT;",
                    )
                    .await
                    .map_err(|e| MemoryError::database(format!("Migration v1 failed: {}", e)))?;
                }
                2 => {
                    // Added expires_at to concepts
                    if !self
                        .column_exists(conn, "csm_concepts", "expires_at")
                        .await?
                    {
                        conn.execute("ALTER TABLE csm_concepts ADD COLUMN expires_at INTEGER", ())
                            .await
                            .map_err(|e| {
                                MemoryError::database(format!("Migration v2 failed: {}", e))
                            })?;
                    }
                }
                3 => {
                    // Added canonical_concept_ids_json to concepts
                    if !self
                        .column_exists(conn, "csm_concepts", "canonical_concept_ids_json")
                        .await?
                    {
                        conn.execute(
                            "ALTER TABLE csm_concepts ADD COLUMN canonical_concept_ids_json TEXT",
                            (),
                        )
                        .await
                        .map_err(|e| {
                            MemoryError::database(format!("Migration v3 failed: {}", e))
                        })?;
                    }
                }
                4 => {
                    // Added canonical concepts table
                    conn.execute_batch(
                        "CREATE TABLE IF NOT EXISTS csm_canonical (
                            id TEXT NOT NULL,
                            version INTEGER NOT NULL,
                            labels_json TEXT NOT NULL,
                            related_json TEXT NOT NULL,
                            PRIMARY KEY (id, version)
                        );",
                    )
                    .await
                    .map_err(|e| MemoryError::database(format!("Migration v4 failed: {}", e)))?;
                }
                5 => {
                    self.apply_v5_namespace_migration(conn).await?;
                }
                _ => {}
            }
            current_version += 1;
            conn.execute(
                "INSERT OR REPLACE INTO csm_schema_version (version) VALUES (?1)",
                params![current_version],
            )
            .await
            .map_err(|e| {
                MemoryError::database(format!("Failed to update schema version: {}", e))
            })?;
        }
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
            .map_err(|e| MemoryError::database(format!("Failed to get schema version: {}", e)))?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch schema version row: {}", e))
        })? {
            let version: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse schema version: {}", e))
            })?;
            Ok(version)
        } else {
            Ok(0)
        }
    }
}
