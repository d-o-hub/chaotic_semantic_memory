//! Persistence layer using libSQL (SQLite/Turso). Auto-migrations, version retention, FK enabled.

// Casts are intentional for schema version math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;
use libsql::{Builder, Connection, Database, params};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub(crate) const LATEST_SCHEMA_VERSION: i64 = 7;

#[derive(Debug)]
pub struct Persistence {
    pub(crate) db: Arc<Database>,
    pub(crate) local_path: Option<String>,
    pub(crate) remote_limit: Option<Arc<Semaphore>>,
    pub(crate) version_retention: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConceptVersion {
    pub concept_id: String,
    pub version: i64,
    pub vector: HVec10240,
    pub metadata: serde_json::Value,
    pub modified_at: u64,
}

impl Persistence {
    /// Create new persistence layer with local SQLite
    pub async fn new_local(path: &str) -> Result<Self> {
        Self::new_local_with_retention(path, 10).await
    }

    pub async fn new_local_with_retention(path: &str, version_retention: usize) -> Result<Self> {
        let db = Builder::new_local(path)
            .build()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to open database: {}", e)))?;

        let persistence = Self {
            db: Arc::new(db),
            local_path: Some(path.to_string()),
            remote_limit: None,
            version_retention: version_retention.max(1),
        };
        persistence.init_schema().await?;
        Ok(persistence)
    }

    /// Create new persistence layer with remote Turso
    pub async fn new_turso(url: &str, token: &str) -> Result<Self> {
        Self::new_turso_with_pool(url, token, 10).await
    }

    pub async fn new_turso_with_pool(url: &str, token: &str, pool_size: usize) -> Result<Self> {
        Self::new_turso_with_pool_and_retention(url, token, pool_size, 10).await
    }

    pub async fn new_turso_with_pool_and_retention(
        url: &str,
        token: &str,
        pool_size: usize,
        version_retention: usize,
    ) -> Result<Self> {
        let db = Builder::new_remote(url.to_string(), token.to_string())
            .build()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to open remote database: {}", e)))?;

        let persistence = Self {
            db: Arc::new(db),
            local_path: None,
            remote_limit: Some(Arc::new(Semaphore::new(pool_size.max(1)))),
            version_retention: version_retention.max(1),
        };
        persistence.init_schema().await?;
        Ok(persistence)
    }

    pub(crate) async fn connect(&self) -> Result<Connection> {
        let conn = self
            .db
            .connect()
            .map_err(|e| MemoryError::database(format!("Failed to connect: {e}")))?;

        if self.local_path.is_some() {
            let _ = conn
                .query("PRAGMA journal_mode=WAL;", ())
                .await
                .map_err(|e| MemoryError::database(format!("Failed to enable WAL mode: {e}")))?;
        }
        conn.execute("PRAGMA foreign_keys=ON;", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to enable foreign keys: {e}")))?;
        Ok(conn)
    }

    pub(crate) async fn acquire_remote_slot(&self) -> Result<Option<OwnedSemaphorePermit>> {
        match &self.remote_limit {
            Some(limit) => {
                limit.clone().acquire_owned().await.map(Some).map_err(|e| {
                    MemoryError::database(format!("Failed to acquire pool slot: {}", e))
                })
            }
            None => Ok(None),
        }
    }

    /// Initialize database schema
    pub(crate) async fn init_schema(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS csm_concepts (
                id TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS csm_associations (
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                strength REAL NOT NULL,
                PRIMARY KEY (from_id, to_id),
                FOREIGN KEY (from_id) REFERENCES csm_concepts(id),
                FOREIGN KEY (to_id) REFERENCES csm_concepts(id)
            );
            CREATE INDEX IF NOT EXISTS idx_csm_associations_from ON csm_associations(from_id);
            CREATE TABLE IF NOT EXISTS csm_versions (
                concept_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                vector BLOB NOT NULL,
                metadata TEXT NOT NULL,
                modified_at INTEGER NOT NULL,
                PRIMARY KEY (concept_id, version),
                FOREIGN KEY (concept_id) REFERENCES csm_concepts(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS csm_schema_version (
                version INTEGER PRIMARY KEY
            );
            INSERT OR IGNORE INTO csm_schema_version(version) VALUES (1);
            COMMIT;",
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to initialize schema: {}", e)))?;
        // Use internal method that reuses the connection to avoid semaphore deadlock
        self.apply_migrations_with_conn(&conn, LATEST_SCHEMA_VERSION)
            .await?;
        Ok(())
    }

    /// Save a concept to the database
    pub async fn save_concept(&self, concept: &Concept) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;
        let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
        let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;

        conn.execute(
            "INSERT OR REPLACE INTO csm_concepts
             (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                concept.id.clone(),
                vector_bytes,
                metadata_json,
                concept.created_at as i64,
                concept.modified_at as i64,
                expires_at,
                canonical_concept_ids_json
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept: {}", e)))?;

        self.record_concept_version(&conn, concept).await?;
        Ok(())
    }

    /// Save concepts in a single transaction
    pub async fn save_concepts(&self, concepts: &[Concept]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {}", e)))?;

        let mut first_error: Option<MemoryError> = None;
        for concept in concepts {
            let vector_bytes = concept.vector.to_bytes();
            let metadata_json = serde_json::to_string(&concept.metadata)?;
            let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
            let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;

            if let Err(e) = conn
                .execute(
                    "INSERT OR REPLACE INTO csm_concepts
                     (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        concept.id.clone(),
                        vector_bytes,
                        metadata_json,
                        concept.created_at as i64,
                        concept.modified_at as i64,
                        expires_at,
                        canonical_concept_ids_json
                    ],
                )
                .await
            {
                first_error = Some(MemoryError::database(format!(
                    "Failed to batch save concept: {}",
                    e
                )));
                break;
            }

            if let Err(e) = self.record_concept_version(&conn, concept).await {
                first_error = Some(e);
                break;
            }
        }

        if let Some(error) = first_error {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(error);
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Load a concept from the database
    pub async fn load_concept(&self, id: &str) -> Result<Option<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json
                 FROM csm_concepts WHERE id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concept: {}", e)))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {}", e)))?
        {
            let vector_bytes: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {}", e)))?;
            let metadata_json: String = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {}", e)))?;
            let created_at: i64 = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get created_at: {}", e)))?;
            let modified_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {}", e)))?;
            let expires_at: Option<i64> = row.get(4).ok();
            let canonical_concept_ids_json: Option<String> = row.get(5).ok();

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;
            let canonical_concept_ids = canonical_concept_ids_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();

            Ok(Some(Concept {
                id: id.to_string(),
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
                expires_at: expires_at.map(|t| t as u64),
                canonical_concept_ids,
            }))
        } else {
            Ok(None)
        }
    }

    /// Load all concepts from the database
    pub async fn load_all_concepts(&self) -> Result<Vec<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json
                 FROM csm_concepts",
                (),
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load concepts: {}", e)))?;

        let mut concepts = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {}", e)))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get id: {}", e)))?;
            let vector_bytes: Vec<u8> = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get vector: {}", e)))?;
            let metadata_json: String = row
                .get(2)
                .map_err(|e| MemoryError::database(format!("Failed to get metadata: {}", e)))?;
            let created_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::database(format!("Failed to get created_at: {}", e)))?;
            let modified_at: i64 = row
                .get(4)
                .map_err(|e| MemoryError::database(format!("Failed to get modified_at: {}", e)))?;
            let expires_at: Option<i64> = row.get(5).ok();
            let canonical_concept_ids_json: Option<String> = row.get(6).ok();

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;
            let canonical_concept_ids = canonical_concept_ids_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_default();

            concepts.push(Concept {
                id,
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
                expires_at: expires_at.map(|t| t as u64),
                canonical_concept_ids,
            });
        }

        Ok(concepts)
    }

    /// Delete a concept from the database
    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {}", e)))?;

        if let Err(e) = conn
            .execute(
                "DELETE FROM csm_associations WHERE from_id = ?1 OR to_id = ?1",
                params![id],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete associations: {}",
                e
            )));
        }

        if let Err(e) = conn
            .execute("DELETE FROM csm_concepts WHERE id = ?1", params![id])
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::database(format!(
                "Failed to delete concept: {}",
                e
            )));
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Save an association
    pub async fn save_association(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        conn.execute(
            "INSERT OR REPLACE INTO csm_associations (from_id, to_id, strength)
             VALUES (?1, ?2, ?3)",
            params![from, to, strength],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save association: {}", e)))?;

        Ok(())
    }

    /// Load associations for a concept
    pub async fn load_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT to_id, strength FROM csm_associations WHERE from_id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load associations: {}", e)))?;

        let mut associations = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {}", e)))?
        {
            let to_id: String = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get to_id: {}", e)))?;
            let strength: f64 = row
                .get(1)
                .map_err(|e| MemoryError::database(format!("Failed to get strength: {}", e)))?;
            associations.push((to_id, strength as f32));
        }

        Ok(associations)
    }

    /// Perform database checkpoint (optimize)
    pub async fn checkpoint(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query("PRAGMA wal_checkpoint(TRUNCATE);", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to checkpoint: {}", e)))?;
        let _ = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to read checkpoint row: {}", e)))?;

        Ok(())
    }

    /// Get database size in bytes
    pub async fn size(&self) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                (),
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to get size: {}", e)))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch row: {}", e)))?
        {
            let size: i64 = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get size value: {}", e)))?;
            Ok(size as u64)
        } else {
            Ok(0)
        }
    }
}
