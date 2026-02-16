//! Persistence layer using libSQL
//!
//! Supports both local SQLite files and remote Turso databases.

use libsql::{params, Builder, Connection, Database};
use std::sync::Arc;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;

/// Database connection manager
pub struct Persistence {
    db: Arc<Database>,
}

impl Persistence {
    /// Create new persistence layer with local SQLite
    pub async fn new_local(path: &str) -> Result<Self> {
        let db = Builder::new_local(path)
            .build()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to open database: {}", e)))?;

        let persistence = Self { db: Arc::new(db) };
        persistence.init_schema().await?;
        Ok(persistence)
    }

    /// Create new persistence layer with remote Turso
    pub async fn new_turso(url: &str, token: &str) -> Result<Self> {
        let db = Builder::new_remote(url.to_string(), token.to_string())
            .build()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to open remote database: {}", e)))?;

        let persistence = Self { db: Arc::new(db) };
        persistence.init_schema().await?;
        Ok(persistence)
    }

    async fn connect(&self) -> Result<Connection> {
        let conn = self
            .db
            .connect()
            .map_err(|e| MemoryError::Database(format!("Failed to connect: {}", e)))?;

        conn.execute("PRAGMA foreign_keys = ON;", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to enable foreign keys: {}", e)))?;

        Ok(conn)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        let conn = self.connect().await?;

        conn.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS concepts (
                id TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS associations (
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                strength REAL NOT NULL,
                PRIMARY KEY (from_id, to_id),
                FOREIGN KEY (from_id) REFERENCES concepts(id),
                FOREIGN KEY (to_id) REFERENCES concepts(id)
            );
            CREATE INDEX IF NOT EXISTS idx_associations_from ON associations(from_id);
            COMMIT;",
        )
        .await
        .map_err(|e| MemoryError::Database(format!("Failed to initialize schema: {}", e)))?;

        Ok(())
    }

    /// Save a concept to the database
    pub async fn save_concept(&self, concept: &Concept) -> Result<()> {
        let conn = self.connect().await?;
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO concepts (id, vector, metadata, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                concept.id.clone(),
                vector_bytes,
                metadata_json,
                concept.created_at as i64,
                concept.modified_at as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::Database(format!("Failed to save concept: {}", e)))?;

        Ok(())
    }

    /// Save concepts in a single transaction
    pub async fn save_concepts(&self, concepts: &[Concept]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to begin transaction: {}", e)))?;

        let mut first_error: Option<MemoryError> = None;
        for concept in concepts {
            let vector_bytes = concept.vector.to_bytes();
            let metadata_json = serde_json::to_string(&concept.metadata)?;

            if let Err(e) = conn
                .execute(
                    "INSERT OR REPLACE INTO concepts (id, vector, metadata, created_at, modified_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        concept.id.clone(),
                        vector_bytes,
                        metadata_json,
                        concept.created_at as i64,
                        concept.modified_at as i64
                    ],
                )
                .await
            {
                first_error = Some(MemoryError::Database(format!(
                    "Failed to batch save concept: {}",
                    e
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
            .map_err(|e| MemoryError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Load a concept from the database
    pub async fn load_concept(&self, id: &str) -> Result<Option<Concept>> {
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT vector, metadata, created_at, modified_at FROM concepts WHERE id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to load concept: {}", e)))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to fetch row: {}", e)))?
        {
            let vector_bytes: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::Database(format!("Failed to get vector: {}", e)))?;
            let metadata_json: String = row
                .get(1)
                .map_err(|e| MemoryError::Database(format!("Failed to get metadata: {}", e)))?;
            let created_at: i64 = row
                .get(2)
                .map_err(|e| MemoryError::Database(format!("Failed to get created_at: {}", e)))?;
            let modified_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::Database(format!("Failed to get modified_at: {}", e)))?;

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;

            Ok(Some(Concept {
                id: id.to_string(),
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
            }))
        } else {
            Ok(None)
        }
    }

    /// Load all concepts from the database
    pub async fn load_all_concepts(&self) -> Result<Vec<Concept>> {
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT id, vector, metadata, created_at, modified_at FROM concepts",
                (),
            )
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to load concepts: {}", e)))?;

        let mut concepts = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to fetch row: {}", e)))?
        {
            let id: String = row
                .get(0)
                .map_err(|e| MemoryError::Database(format!("Failed to get id: {}", e)))?;
            let vector_bytes: Vec<u8> = row
                .get(1)
                .map_err(|e| MemoryError::Database(format!("Failed to get vector: {}", e)))?;
            let metadata_json: String = row
                .get(2)
                .map_err(|e| MemoryError::Database(format!("Failed to get metadata: {}", e)))?;
            let created_at: i64 = row
                .get(3)
                .map_err(|e| MemoryError::Database(format!("Failed to get created_at: {}", e)))?;
            let modified_at: i64 = row
                .get(4)
                .map_err(|e| MemoryError::Database(format!("Failed to get modified_at: {}", e)))?;

            let vector = HVec10240::from_bytes(&vector_bytes)?;
            let metadata = serde_json::from_str(&metadata_json)?;

            concepts.push(Concept {
                id,
                vector,
                metadata,
                created_at: created_at as u64,
                modified_at: modified_at as u64,
            });
        }

        Ok(concepts)
    }

    /// Delete a concept from the database
    pub async fn delete_concept(&self, id: &str) -> Result<()> {
        let conn = self.connect().await?;

        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to begin transaction: {}", e)))?;

        if let Err(e) = conn
            .execute(
                "DELETE FROM associations WHERE from_id = ?1 OR to_id = ?1",
                params![id],
            )
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::Database(format!(
                "Failed to delete associations: {}",
                e
            )));
        }

        if let Err(e) = conn
            .execute("DELETE FROM concepts WHERE id = ?1", params![id])
            .await
        {
            let _ = conn.execute("ROLLBACK", ()).await;
            return Err(MemoryError::Database(format!(
                "Failed to delete concept: {}",
                e
            )));
        }

        conn.execute("COMMIT", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Save an association
    pub async fn save_association(&self, from: &str, to: &str, strength: f32) -> Result<()> {
        let conn = self.connect().await?;

        conn.execute(
            "INSERT OR REPLACE INTO associations (from_id, to_id, strength)
             VALUES (?1, ?2, ?3)",
            params![from, to, strength],
        )
        .await
        .map_err(|e| MemoryError::Database(format!("Failed to save association: {}", e)))?;

        Ok(())
    }

    /// Save associations in a single transaction
    pub async fn save_associations(&self, associations: &[(String, String, f32)]) -> Result<()> {
        if associations.is_empty() {
            return Ok(());
        }

        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to begin transaction: {}", e)))?;

        let mut first_error: Option<MemoryError> = None;
        for (from, to, strength) in associations {
            if let Err(e) = conn
                .execute(
                    "INSERT OR REPLACE INTO associations (from_id, to_id, strength)
                     VALUES (?1, ?2, ?3)",
                    params![from.clone(), to.clone(), *strength],
                )
                .await
            {
                first_error = Some(MemoryError::Database(format!(
                    "Failed to batch save association: {}",
                    e
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
            .map_err(|e| MemoryError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Load associations for a concept
    pub async fn load_associations(&self, id: &str) -> Result<Vec<(String, f32)>> {
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT to_id, strength FROM associations WHERE from_id = ?1",
                params![id],
            )
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to load associations: {}", e)))?;

        let mut associations = Vec::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to fetch row: {}", e)))?
        {
            let to_id: String = row
                .get(0)
                .map_err(|e| MemoryError::Database(format!("Failed to get to_id: {}", e)))?;
            let strength: f64 = row
                .get(1)
                .map_err(|e| MemoryError::Database(format!("Failed to get strength: {}", e)))?;
            associations.push((to_id, strength as f32));
        }

        Ok(associations)
    }

    /// Perform database checkpoint (optimize)
    pub async fn checkpoint(&self) -> Result<()> {
        let conn = self.connect().await?;

        let mut rows = conn
            .query("PRAGMA wal_checkpoint(TRUNCATE);", ())
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to checkpoint: {}", e)))?;
        let _ = rows
            .next()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to read checkpoint row: {}", e)))?;

        Ok(())
    }

    /// Get database size in bytes
    pub async fn size(&self) -> Result<u64> {
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                (),
            )
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to get size: {}", e)))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::Database(format!("Failed to fetch row: {}", e)))?
        {
            let size: i64 = row
                .get(0)
                .map_err(|e| MemoryError::Database(format!("Failed to get size value: {}", e)))?;
            Ok(size as u64)
        } else {
            Ok(0)
        }
    }
}
