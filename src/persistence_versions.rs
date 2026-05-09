use crate::error::{MemoryError, Result};
use crate::persistence::Persistence;
use crate::singularity::Concept;
use crate::persistence::ConceptVersion;
use crate::hyperdim::Hypervector;
use libsql::{Connection, params};

impl Persistence {
    pub(crate) async fn record_concept_version<H: Hypervector + 'static>(
        &self,
        conn: &Connection,
        ns: &str,
        concept: &Concept<H>,
    ) -> Result<()> {
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2",
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
        let format = H::format_name();

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
                format
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

    pub async fn get_concept_history<H: Hypervector + 'static>(
        &self,
        ns: &str,
        id: &str,
        limit: usize,
    ) -> Result<Vec<ConceptVersion<H>>> {
        let _permit = self.acquire_remote_slot().await?;
        let conn = self.connect().await?;

        let mut rows = conn
            .query(
                "SELECT version, vector, metadata, modified_at, vector_format
                 FROM csm_versions WHERE namespace = ?1 AND concept_id = ?2
                 ORDER BY version DESC LIMIT ?3",
                params![ns.to_string(), id.to_string(), limit as i64],
            )
            .await
            .map_err(|e| MemoryError::database(format!("Failed to query history: {e}")))?;

        let mut history = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| MemoryError::database(format!("Fetch row fail: {e}")))? {
            let version: i64 = row.get(0).map_err(|e| MemoryError::database(format!("Get version fail: {e}")))?;
            let vector_bytes: Vec<u8> = row.get(1).map_err(|e| MemoryError::database(format!("Get vector fail: {e}")))?;
            let metadata_json: String = row.get(2).map_err(|e| MemoryError::database(format!("Get metadata fail: {e}")))?;
            let modified_at: i64 = row.get(3).map_err(|e| MemoryError::database(format!("Get modified_at fail: {e}")))?;
            let vector_format: String = row.get(4).unwrap_or_else(|_| "f32".to_string());

            let vector = if vector_format == "binary" {
                #[cfg(feature = "hv-binary")]
                {
                    use crate::hyperdim::BHVec10240;
                    if std::any::TypeId::of::<H>() == std::any::TypeId::of::<BHVec10240>() {
                        H::from_bytes(&vector_bytes)?
                    } else {
                        let bhv = BHVec10240::from_bytes(&vector_bytes)?;
                        let fhv = bhv.to_f32();
                        H::from_hvec(&fhv)
                    }
                }
                #[cfg(not(feature = "hv-binary"))]
                { return Err(MemoryError::UnsupportedOperation("hv-binary disabled".to_string())); }
            } else {
                H::from_bytes(&vector_bytes)?
            };

            history.push(ConceptVersion {
                concept_id: id.to_string(),
                version,
                vector,
                metadata: serde_json::from_str(&metadata_json)?,
                modified_at: modified_at as u64,
            });
        }
        Ok(history)
    }
}
