//! ANN index persistence operations.
//!
//! Extracted from persistence.rs to satisfy the 500 LOC gate.

use crate::persistence::Persistence;
use csm_core::error::{MemoryError, Result};
use libsql::params;

impl Persistence {
    /// Save the serialized index state to the database.
    pub async fn save_index(&self, ns: &str, id: &str, data: &[u8]) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "INSERT OR REPLACE INTO csm_hnsw_graph (namespace, id, data, modified_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                ns.to_string(),
                id,
                data,
                crate::singularity::unix_now_secs() as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save index: {e}")))?;
        Ok(())
    }

    /// Load the serialized index state from the database.
    pub async fn load_index(&self, ns: &str, id: &str) -> Result<Option<Vec<u8>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        let mut rows = conn
            .query(
                "SELECT data FROM csm_hnsw_graph WHERE namespace = ?1 AND id = ?2",
                params![ns.to_string(), id],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to load index: {e}")))?;

        if let Some(row) = rows
            .next()
            .await
            .map_err(|e| MemoryError::database(format!("Failed to fetch index row: {e}")))?
        {
            let data: Vec<u8> = row
                .get(0)
                .map_err(|e| MemoryError::database(format!("Failed to get index data: {e}")))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_index_roundtrip() {
        let path = "/tmp/test_index_persist_rt.db";
        let persistence = Persistence::new_local(path).await.unwrap();

        let test_data = vec![1u8, 2, 3, 4, 5, 42, 99];
        persistence
            .save_index("default", "test-idx", &test_data)
            .await
            .unwrap();

        let loaded = persistence.load_index("default", "test-idx").await.unwrap();
        assert_eq!(loaded, Some(test_data));

        // Non-existent index returns None
        let missing = persistence
            .load_index("default", "no-such-idx")
            .await
            .unwrap();
        assert_eq!(missing, None);

        std::fs::remove_file(path).ok();
    }
}
