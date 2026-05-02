//! Persistence layer using libSQL (SQLite/Turso). Auto-migrations, version retention, FK enabled.
use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;
use libsql::{Builder, Connection, Database, params};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub(crate) const LATEST_SCHEMA_VERSION: i64 = 6;

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
    pub async fn new_local(path: &str) -> Result<Self> {
        Self::new_local_with_retention(path, 10).await
    }

    pub async fn new_local_with_retention(path: &str, version_retention: usize) -> Result<Self> {
        let db = Builder::new_local(path)
            .build()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        let persistence = Self {
            db: Arc::new(db),
            local_path: Some(path.to_string()),
            remote_limit: None,
            version_retention: version_retention.max(1),
        };
        persistence.init_schema().await?;
        Ok(persistence)
    }

    pub async fn new_turso(url: &str, token: &str) -> Result<Self> {
        Self::new_turso_with_pool_and_retention(url, token, 10, 10).await
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
            .map_err(|e| MemoryError::database(e.to_string()))?;
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
            .map_err(|e| MemoryError::database(e.to_string()))?;
        if self.local_path.is_some() {
            let _ = conn.query("PRAGMA journal_mode=WAL;", ()).await;
        }
        conn.execute("PRAGMA foreign_keys=ON;", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(conn)
    }

    pub(crate) async fn acquire_remote_slot(&self) -> Result<Option<OwnedSemaphorePermit>> {
        match &self.remote_limit {
            Some(limit) => limit
                .clone()
                .acquire_owned()
                .await
                .map(Some)
                .map_err(|e| MemoryError::database(e.to_string())),
            None => Ok(None),
        }
    }

    pub(crate) async fn init_schema(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute_batch("BEGIN; CREATE TABLE IF NOT EXISTS csm_concepts (id TEXT PRIMARY KEY, vector BLOB NOT NULL, metadata TEXT NOT NULL, created_at INTEGER NOT NULL, modified_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS csm_associations (from_id TEXT NOT NULL, to_id TEXT NOT NULL, strength REAL NOT NULL, PRIMARY KEY (from_id, to_id), FOREIGN KEY (from_id) REFERENCES csm_concepts(id), FOREIGN KEY (to_id) REFERENCES csm_concepts(id)); CREATE INDEX IF NOT EXISTS idx_csm_associations_from ON csm_associations(from_id); CREATE TABLE IF NOT EXISTS csm_versions (concept_id TEXT NOT NULL, version INTEGER NOT NULL, vector BLOB NOT NULL, metadata TEXT NOT NULL, modified_at INTEGER NOT NULL, PRIMARY KEY (concept_id, version), FOREIGN KEY (concept_id) REFERENCES csm_concepts(id) ON DELETE CASCADE); CREATE TABLE IF NOT EXISTS csm_metrics (key TEXT PRIMARY KEY, value INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS csm_schema_version (version INTEGER PRIMARY KEY); INSERT OR IGNORE INTO csm_schema_version(version) VALUES (1); COMMIT;").await.map_err(|e| MemoryError::database(e.to_string()))?;
        self.apply_migrations_with_conn(&conn, LATEST_SCHEMA_VERSION)
            .await?;
        Ok(())
    }

    pub async fn save_concept(&self, concept: &Concept) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;
        let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
        let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;
        conn.execute("INSERT OR REPLACE INTO csm_concepts (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![concept.id.clone(), vector_bytes, metadata_json, concept.created_at as i64, concept.modified_at as i64, expires_at, canonical_concept_ids_json]).await.map_err(|e| MemoryError::database(e.to_string()))?;
        self.record_concept_version(&conn, concept).await?;
        Ok(())
    }

    pub async fn load_concept(&self, id: &str) -> Result<Option<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn.query("SELECT vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json FROM csm_concepts WHERE id = ?1", params![id]).await.map_err(|e| MemoryError::database(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?
        {
            let vector = HVec10240::from_bytes(
                &row.get::<Vec<u8>>(0)
                    .map_err(|e| MemoryError::database(e.to_string()))?,
            )?;
            let metadata = serde_json::from_str(
                &row.get::<String>(1)
                    .map_err(|e| MemoryError::database(e.to_string()))?,
            )?;
            let expires_at = row.get::<Option<i64>>(4).ok().flatten().map(|t| t as u64);
            let canonical_concept_ids = row
                .get::<Option<String>>(5)
                .ok()
                .flatten()
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default();
            Ok(Some(Concept {
                id: id.to_string(),
                vector,
                metadata,
                created_at: row
                    .get::<i64>(2)
                    .map_err(|e| MemoryError::database(e.to_string()))?
                    as u64,
                modified_at: row
                    .get::<i64>(3)
                    .map_err(|e| MemoryError::database(e.to_string()))?
                    as u64,
                expires_at,
                canonical_concept_ids,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        conn.execute(
            "DELETE FROM csm_associations WHERE from_id = ?1 OR to_id = ?1",
            params![id],
        )
        .await
        .map_err(|e| MemoryError::database(e.to_string()))?;
        conn.execute("DELETE FROM csm_concepts WHERE id = ?1", params![id])
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn save_association(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("INSERT OR REPLACE INTO csm_associations (from_id, to_id, strength) VALUES (?1, ?2, ?3)", params![from, to, strength]).await.map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn load_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT to_id, strength FROM csm_associations WHERE from_id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        let mut associations = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?
        {
            associations.push((
                row.get::<String>(0)
                    .map_err(|e| MemoryError::database(e.to_string()))?,
                row.get::<f64>(1)
                    .map_err(|e| MemoryError::database(e.to_string()))? as f32,
            ));
        }
        Ok(associations)
    }

    pub async fn checkpoint(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query("PRAGMA wal_checkpoint(TRUNCATE);", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        let _ = rows.next().await;
        Ok(())
    }

    pub async fn size(&self) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                (),
            )
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?
        {
            Ok(row
                .get::<i64>(0)
                .map_err(|e| MemoryError::database(e.to_string()))? as u64)
        } else {
            Ok(0)
        }
    }

    pub async fn health_check(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query("SELECT 1", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        let _ = rows.next().await;
        Ok(())
    }

    pub async fn save_metric(&self, key: &str, value: u64) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "INSERT OR REPLACE INTO csm_metrics (key, value) VALUES (?1, ?2)",
            params![key, value as i64],
        )
        .await
        .map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn load_metric(&self, key: &str) -> Result<u64> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query("SELECT value FROM csm_metrics WHERE key = ?1", params![key])
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?
        {
            Ok(row
                .get::<i64>(0)
                .map_err(|e| MemoryError::database(e.to_string()))? as u64)
        } else {
            Ok(0)
        }
    }

    pub async fn clear_metrics(&self) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("DELETE FROM csm_metrics", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn save_concepts(&self, concepts: &[Concept]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        for concept in concepts {
            let vector_bytes = concept.vector.to_bytes();
            let metadata_json = serde_json::to_string(&concept.metadata)?;
            let expires_at: Option<i64> = concept.expires_at.map(|t| t as i64);
            let canonical_concept_ids_json = serde_json::to_string(&concept.canonical_concept_ids)?;
            conn.execute("INSERT OR REPLACE INTO csm_concepts (id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![concept.id.clone(), vector_bytes, metadata_json, concept.created_at as i64, concept.modified_at as i64, expires_at, canonical_concept_ids_json]).await.map_err(|e| MemoryError::database(e.to_string()))?;
            self.record_concept_version(&conn, concept).await?;
        }
        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?;
        Ok(())
    }

    pub async fn load_all_concepts(&self) -> Result<Vec<Concept>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn.query("SELECT id, vector, metadata, created_at, modified_at, expires_at, canonical_concept_ids_json FROM csm_concepts", ()).await.map_err(|e| MemoryError::database(e.to_string()))?;
        let mut concepts = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(e.to_string()))?
        {
            let id = row
                .get::<String>(0)
                .map_err(|e| MemoryError::database(e.to_string()))?;
            let vector = HVec10240::from_bytes(
                &row.get::<Vec<u8>>(1)
                    .map_err(|e| MemoryError::database(e.to_string()))?,
            )?;
            let metadata = serde_json::from_str(
                &row.get::<String>(2)
                    .map_err(|e| MemoryError::database(e.to_string()))?,
            )?;
            let expires_at = row.get::<Option<i64>>(5).ok().flatten().map(|t| t as u64);
            let canonical_concept_ids = row
                .get::<Option<String>>(6)
                .ok()
                .flatten()
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or_default();
            concepts.push(Concept {
                id,
                vector,
                metadata,
                created_at: row
                    .get::<i64>(3)
                    .map_err(|e| MemoryError::database(e.to_string()))?
                    as u64,
                modified_at: row
                    .get::<i64>(4)
                    .map_err(|e| MemoryError::database(e.to_string()))?
                    as u64,
                expires_at,
                canonical_concept_ids,
            });
        }
        Ok(concepts)
    }
}
