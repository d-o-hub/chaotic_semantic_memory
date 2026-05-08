use crate::error::{MemoryError, Result};
use crate::hyperdim::Hypervector;
use crate::persistence::Persistence;
use crate::singularity::Concept;
use libsql::{Connection, params};

impl Persistence {
    pub(crate) async fn record_concept_version<H: Hypervector>(
        &self,
        conn: &Connection,
        concept: &Concept<H>,
    ) -> Result<()> {
        self.record_concept_version_scoped(conn, "_default", concept)
            .await
    }

    pub(crate) async fn record_concept_version_scoped<H: Hypervector>(
        &self,
        conn: &Connection,
        ns: &str,
        concept: &Concept<H>,
    ) -> Result<()> {
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version),0) FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
                params![ns.to_string(), concept.id.clone()],
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
        let vector_bytes = concept.vector.to_bytes();
        let metadata_json = serde_json::to_string(&concept.metadata)?;

        conn.execute(
            "INSERT INTO csm_versions (namespace, concept_id, version, vector, metadata, modified_at, vector_format)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ns.to_string(),
                concept.id.clone(),
                next_version,
                vector_bytes,
                metadata_json,
                concept.modified_at as i64,
                H::format_name()
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
            params![
                ns.to_string(),
                concept.id.clone(),
                self.version_retention as i64
            ],
        )
        .await
        .map_err(|e| MemoryError::database(format!("Failed to prune concept versions: {e}")))?;

        Ok(())
    }
}
