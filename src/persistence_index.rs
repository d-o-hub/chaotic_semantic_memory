use libsql::params;

use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;

impl Persistence {
    /// Perform database checkpoint (optimize).
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

    /// Save the serialized index state to the database.
    pub async fn save_index(&self, id: &str, data: &[u8]) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "INSERT OR REPLACE INTO csm_hnsw_graph (id, data, modified_at)
             VALUES (?1, ?2, ?3)",
            params![id, data, crate::singularity::unix_now_secs() as i64],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save index: {}", e)))?;
        Ok(())
    }

    /// Load the serialized index state from the database.
    pub async fn load_index(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query("SELECT data FROM csm_hnsw_graph WHERE id = ?1", params![id])
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load index: {}", e)))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch index row: {}", e)))?
        {
            let data: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get index data: {}", e)))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Get database size in bytes.
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
