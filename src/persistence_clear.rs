//! Persistence clear/delete operations.
//!
//! Extracted from persistence_ops.rs for LOC gate compliance.

use libsql::params;

use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;

impl Persistence {
    pub async fn clear_namespace(&self, ns: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute("BEGIN", ())
            .await
            .map_err(|e| MemoryError::database(format!("Failed to begin transaction: {e}")))?;

        let mut first_error: Option<MemoryError> = None;
        for table in [
            "csm_associations",
            "csm_versions",
            "csm_concepts",
            "csm_hnsw_graph",
            "csm_canonical",
        ] {
            let query = format!("DELETE FROM {table} WHERE namespace = ?1");
            if let Err(e) = conn.execute(&query, params![ns.to_string()]).await {
                first_error = Some(MemoryError::database(format!(
                    "Failed to clear {table} for namespace: {e}"
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
            .map_err(|e| MemoryError::database(format!("Failed to commit namespace clear: {e}")))?;
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
             DELETE FROM csm_hnsw_graph;
             DELETE FROM csm_canonical;
             COMMIT;",
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear all data: {e}")))?;
        Ok(())
    }

    /// Delete a single association between two concepts.
    pub async fn delete_association(&self, ns: &str, from: &str, to: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE namespace = ?1 AND from_id = ?2 AND to_id = ?3",
            params![ns.to_string(), from, to],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to delete association: {e}")))?;
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    pub async fn clear_concept_associations(&self, ns: &str, id: &str) -> Result<()> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;
        conn.execute(
            "DELETE FROM csm_associations WHERE namespace = ?1 AND from_id = ?2",
            params![ns.to_string(), id],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to clear concept associations: {e}")))?;
        Ok(())
    }
}
