use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;
use libsql::params;

impl Persistence {
    pub(crate) async fn table_exists(
        &self,
        conn: &libsql::Connection,
        table_name: &str,
    ) -> Result<bool> {
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![table_name],
            )
            .await
            .map_err(|e| {
                MemoryError::database(format!("Failed to check table existence: {}", e))
            })?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch table existence: {}", e)))?
        {
            let count: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse table count: {}", e))
            })?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn column_exists(
        &self,
        conn: &libsql::Connection,
        table_name: &str,
        column_name: &str,
    ) -> Result<bool> {
        // Validate table_name to prevent SQL injection in pragma_table_info (CWE-89)
        if !table_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(MemoryError::InvalidInput {
                field: "table_name".to_string(),
                reason: "Table name contains invalid characters".to_string(),
            });
        }

        let sql = format!(
            "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
            table_name
        );

        let mut rows = conn.query(&sql, params![column_name]).await.map_err(|e| {
            MemoryError::database(format!("Failed to check column existence: {}", e))
        })?;

        if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch column existence: {}", e))
        })? {
            let count: i64 = row.get(0).map_err(|e| {
                MemoryError::database(format!("Failed to parse column count: {}", e))
            })?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    pub(crate) async fn apply_v5_namespace_migration(
        &self,
        conn: &libsql::Connection,
    ) -> Result<()> {
        if self.table_exists(conn, "concepts").await? {
            if self.table_exists(conn, "csm_concepts").await? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO csm_concepts (id, vector, metadata, created_at, modified_at)
                     SELECT id, vector, metadata, created_at, modified_at FROM concepts;
                     DROP TABLE concepts;",
                )
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 concepts merge: {}", e))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE concepts RENAME TO csm_concepts;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!("Failed migration v5 concepts rename: {}", e))
                    })?;
            }
        }

        if self.table_exists(conn, "associations").await? {
            if self.table_exists(conn, "csm_associations").await? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO csm_associations (from_id, to_id, strength)
                     SELECT from_id, to_id, strength FROM associations;
                     DROP TABLE associations;",
                )
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 associations merge: {}", e))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE associations RENAME TO csm_associations;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!(
                            "Failed migration v5 associations rename: {}",
                            e
                        ))
                    })?;
            }
        }

        if self.table_exists(conn, "concept_versions").await? {
            if self.table_exists(conn, "csm_versions").await? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO csm_versions (concept_id, version, vector, metadata, modified_at)
                     SELECT concept_id, version, vector, metadata, modified_at FROM concept_versions;
                     DROP TABLE concept_versions;",
                )
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 versions merge: {}", e))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE concept_versions RENAME TO csm_versions;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!("Failed migration v5 versions rename: {}", e))
                    })?;
            }
        }

        if self.table_exists(conn, "canonical_concepts").await? {
            if self.table_exists(conn, "csm_canonical").await? {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO csm_canonical (id, version, labels_json, related_json)
                     SELECT id, version, labels_json, related_json FROM canonical_concepts;
                     DROP TABLE canonical_concepts;",
                )
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 canonical merge: {}", e))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE canonical_concepts RENAME TO csm_canonical;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!(
                            "Failed migration v5 canonical rename: {}",
                            e
                        ))
                    })?;
            }
        }

        if self.table_exists(conn, "__schema_version").await? {
            conn.execute_batch("DROP TABLE __schema_version;")
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 schema cleanup: {}", e))
                })?;
        }

        Ok(())
    }
}
