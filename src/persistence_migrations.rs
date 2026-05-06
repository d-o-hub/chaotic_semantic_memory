use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;
use libsql::params;
use tracing::info;

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
            .map_err(|e| MemoryError::database(format!("Failed to check table existence: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch table existence: {e}")))?
        {
            let count: i64 = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to parse table count: {e}")))?;
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

        let sql = format!("SELECT COUNT(*) FROM pragma_table_info('{table_name}') WHERE name = ?1");

        let mut rows = conn
            .query(&sql, params![column_name])
            .await
            .map_err(|e| MemoryError::database(format!("Failed to check column existence: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch column existence: {e}")))?
        {
            let count: i64 = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to parse column count: {e}")))?;
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
                    MemoryError::database(format!("Failed migration v5 concepts merge: {e}"))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE concepts RENAME TO csm_concepts;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!("Failed migration v5 concepts rename: {e}"))
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
                    MemoryError::database(format!("Failed migration v5 associations merge: {e}"))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE associations RENAME TO csm_associations;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!(
                            "Failed migration v5 associations rename: {e}"
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
                    MemoryError::database(format!("Failed migration v5 versions merge: {e}"))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE concept_versions RENAME TO csm_versions;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!("Failed migration v5 versions rename: {e}"))
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
                    MemoryError::database(format!("Failed migration v5 canonical merge: {e}"))
                })?;
            } else {
                conn.execute_batch("ALTER TABLE canonical_concepts RENAME TO csm_canonical;")
                    .await
                    .map_err(|e| {
                        MemoryError::database(format!("Failed migration v5 canonical rename: {e}"))
                    })?;
            }
        }

        if self.table_exists(conn, "__schema_version").await? {
            conn.execute_batch("DROP TABLE __schema_version;")
                .await
                .map_err(|e| {
                    MemoryError::database(format!("Failed migration v5 schema cleanup: {e}"))
                })?;
        }

        Ok(())
    }

    pub(crate) async fn apply_v8_namespace_migration(
        &self,
        conn: &libsql::Connection,
    ) -> Result<()> {
        // v8: Namespace isolation. Update PKs and FKs to include namespace.
        conn.execute_batch(
            "-- 1. Concepts
             ALTER TABLE csm_concepts RENAME TO csm_concepts_old;
             CREATE TABLE csm_concepts (
                 namespace TEXT NOT NULL DEFAULT '_default',
                 id TEXT NOT NULL,
                 vector BLOB NOT NULL,
                 metadata TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 modified_at INTEGER NOT NULL,
                 expires_at INTEGER,
                 canonical_concept_ids_json TEXT,
                 PRIMARY KEY (namespace, id)
             );
             INSERT INTO csm_concepts (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
             SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json FROM csm_concepts_old;
             DROP TABLE csm_concepts_old;
             CREATE INDEX idx_csm_concepts_namespace ON csm_concepts(namespace);

             -- 2. Associations
             ALTER TABLE csm_associations RENAME TO csm_associations_old;
             CREATE TABLE csm_associations (
                 namespace TEXT NOT NULL DEFAULT '_default',
                 from_id TEXT NOT NULL,
                 to_id TEXT NOT NULL,
                 strength REAL NOT NULL,
                 PRIMARY KEY (namespace, from_id, to_id),
                 FOREIGN KEY (namespace, from_id) REFERENCES csm_concepts(namespace, id),
                 FOREIGN KEY (namespace, to_id) REFERENCES csm_concepts(namespace, id)
             );
             INSERT INTO csm_associations (from_id, to_id, strength)
             SELECT from_id, to_id, strength FROM csm_associations_old;
             DROP TABLE csm_associations_old;
             CREATE INDEX idx_csm_associations_namespace ON csm_associations(namespace);
             CREATE INDEX idx_csm_associations_from ON csm_associations(namespace, from_id);

             -- 3. Versions
             ALTER TABLE csm_versions RENAME TO csm_versions_old;
             CREATE TABLE csm_versions (
                 namespace TEXT NOT NULL DEFAULT '_default',
                 concept_id TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 vector BLOB NOT NULL,
                 metadata TEXT NOT NULL,
                 modified_at INTEGER NOT NULL,
                 PRIMARY KEY (namespace, concept_id, version),
                 FOREIGN KEY (namespace, concept_id) REFERENCES csm_concepts(namespace, id)
             );
             INSERT INTO csm_versions (concept_id, version, vector, metadata, modified_at)
             SELECT concept_id, version, vector, metadata, modified_at FROM csm_versions_old;
             DROP TABLE csm_versions_old;
             CREATE INDEX idx_csm_versions_namespace ON csm_versions(namespace);
             CREATE INDEX idx_csm_versions_modified_at ON csm_versions(namespace, modified_at);

             -- 4. HNSW Graph
             ALTER TABLE csm_hnsw_graph RENAME TO csm_hnsw_graph_old;
             CREATE TABLE csm_hnsw_graph (
                 namespace TEXT NOT NULL DEFAULT '_default',
                 id TEXT NOT NULL,
                 data BLOB NOT NULL,
                 modified_at INTEGER NOT NULL,
                 PRIMARY KEY (namespace, id)
             );
             INSERT INTO csm_hnsw_graph (id, data, modified_at)
             SELECT id, data, modified_at FROM csm_hnsw_graph_old;
             DROP TABLE csm_hnsw_graph_old;
             CREATE INDEX idx_csm_hnsw_graph_namespace ON csm_hnsw_graph(namespace);

             -- 5. Canonical Concepts
             ALTER TABLE csm_canonical RENAME TO csm_canonical_old;
             CREATE TABLE csm_canonical (
                 namespace TEXT NOT NULL DEFAULT '_default',
                 id TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 labels_json TEXT NOT NULL,
                 related_json TEXT NOT NULL,
                 PRIMARY KEY (namespace, id)
             );
             INSERT INTO csm_canonical (id, version, labels_json, related_json)
             SELECT id, version, labels_json, related_json FROM csm_canonical_old;
             DROP TABLE csm_canonical_old;
             CREATE INDEX idx_csm_canonical_namespace ON csm_canonical(namespace);",
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed migration v8 namespace isolation: {e}")))?;

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

            if version == 3
                && !self
                    .column_exists(conn, "csm_concepts", "expires_at")
                    .await?
            {
                conn.execute_batch("ALTER TABLE csm_concepts ADD COLUMN expires_at INTEGER;")
                    .await
                    .map_err(|e| MemoryError::database(format!("Failed migration v3: {e}")))?;
            }

            if version == 4 {
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

            if version == 6
                && !self
                    .column_exists(conn, "csm_concepts", "canonical_concept_ids_json")
                    .await?
            {
                conn.execute_batch(
                    "ALTER TABLE csm_concepts ADD COLUMN canonical_concept_ids_json TEXT;",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v6: {e}")))?;
            }

            if version == 7 {
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS csm_hnsw_graph (
                        id TEXT PRIMARY KEY,
                        data BLOB NOT NULL,
                        modified_at INTEGER NOT NULL
                    );",
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed migration v7: {}", e)))?;
            }

            if version == 8 {
                self.apply_v8_namespace_migration(conn).await?;
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
