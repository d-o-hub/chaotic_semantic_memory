//! ANN index persistence operations.
//!
//! Extracted from persistence.rs to satisfy the 500 LOC gate.

use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;
use libsql::params;

impl Persistence {
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

        // #3: Freshness check. We also want the modified_at timestamp to verify
        // if the persisted index matches the concepts in the database.
        let mut rows = conn
            .query(
                "SELECT data, modified_at FROM csm_hnsw_graph WHERE id = ?1",
                params![id],
            )
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

            let modified_at: i64 = row.get(1).map_err(|e| {
                MemoryError::database(format!("Failed to get index modified_at: {}", e))
            })?;

            // Check if there are any concepts modified AFTER the index was last saved.
            // If so, the index is stale and we should return None to trigger a rebuild.
            let mut stale_check = conn
                .query(
                    "SELECT 1 FROM csm_concepts WHERE modified_at > ?1 LIMIT 1",
                    params![modified_at],
                )
                .await
                .map_err(|e| MemoryError::database(format!("Failed stale check: {}", e)))?;

            if stale_check
                .next()
                .await
                .map_err(|e| MemoryError::database(format!("Failed fetch stale row: {}", e)))?
                .is_some()
            {
                return Ok(None);
            }

            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}
