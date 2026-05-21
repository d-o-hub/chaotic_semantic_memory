use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;
use crate::singularity::Concept;
use libsql::{Connection, params};

impl Persistence {
    pub(crate) async fn record_concept_version(
        &self,
        conn: &Connection,
        concept: &Concept,
    ) -> Result<()> {
        self.record_concept_version_scoped(conn, "_default", concept, None, None)
            .await
    }

    /// Records a new concept version, optionally using pre-computed vector bytes and metadata JSON.
    /// Performance Optimization: Accepting pre-computed values avoids redundant serialization and
    /// allocations when this is called from batch operations or normal save paths.
    pub(crate) async fn record_concept_version_scoped(
        &self,
        conn: &Connection,
        ns: &str,
        concept: &Concept,
        vector_bytes: Option<&[u8]>,
        metadata_json: Option<&str>,
    ) -> Result<()> {
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
                params![ns, concept.id.as_str()],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to query concept version: {e}")))?;

        let current = if let Some(row) = rows.next().await.map_err(|e| {
            MemoryError::database(format!("Failed to fetch concept version row: {e}"))
        })? {
            row.get::<i64>(0).map_err(|e| {
                MemoryError::database(format!("Failed to read version from row: {e}"))
            })?
        } else {
            0
        };
        let next_version = current + 1;
        let vector_bytes_owned: Vec<u8>;
        let vector_ref = if let Some(v) = vector_bytes {
            v
        } else {
            vector_bytes_owned = concept.vector.to_bytes();
            &vector_bytes_owned
        };

        let metadata_owned: String;
        let metadata_ref = if let Some(m) = metadata_json {
            m
        } else {
            metadata_owned = serde_json::to_string(&concept.metadata)?;
            &metadata_owned
        };

        conn.execute(
            "INSERT INTO csm_versions (namespace, concept_id, version, vector, metadata, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                ns,
                concept.id.as_str(),
                next_version,
                vector_ref,
                metadata_ref,
                concept.modified_at as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to save concept version: {e}")))?;

        conn.execute(
            "DELETE FROM csm_versions
             WHERE namespace = ?1 AND concept_id = ?2
             AND version <= (
                SELECT MAX(version) - ?3 FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2
             )",
            params![ns, concept.id.as_str(), self.version_retention as i64],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to prune concept versions: {e}")))?;

        Ok(())
    }
}
