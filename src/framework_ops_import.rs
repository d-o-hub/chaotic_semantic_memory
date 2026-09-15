//! Export, import, backup/restore, and concept history operations.
//!
//! Split from `framework_ops.rs` to respect the 500-LOC module ceiling.

use crate::export_payload::{
    BinaryExportPayload, ExportPayload, concept_to_export, export_to_concept, unix_now_secs,
};
use crate::framework::ChaoticSemanticFramework;
use crate::framework_validation::{MAX_IMPORT_SIZE, validate_path};
use crate::singularity::Concept;
use bincode::Options;
use csm_core_lib::error::Result;
use tokio::fs;
use tracing::{instrument, warn};

const MAX_HISTORY_LIMIT: usize = 1000;

impl ChaoticSemanticFramework {
    /// Export memory state to JSON file.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing
                    .all_concepts(&ns)
                    .into_iter()
                    .map(concept_to_export)
                    .collect(),
                associations: sing.all_associations(&ns),
            }
        };
        let data = serde_json::to_vec_pretty(&payload)?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Securely read a file into bytes with size limit (CWE-770).
    pub(crate) async fn secure_read_file(
        &self,
        path: &std::path::Path,
        limit: u64,
    ) -> Result<Vec<u8>> {
        let metadata = fs::metadata(path).await?;
        if metadata.len() > limit {
            return Err(csm_core_lib::error::MemoryError::InvalidInput {
                field: "file_size".to_string(),
                reason: format!(
                    "File size {} exceeds maximum allowed size {}",
                    metadata.len(),
                    limit
                ),
            });
        }
        Ok(fs::read(path).await?)
    }

    /// #526: apply import payload with short write holds.
    ///
    /// Phase 1 (write): inject concepts only.  
    /// Phase 2 (write): associations only so readers can run between phases.
    /// TOCTOU: a concurrent delete between phases may cause associate to fail;
    /// invalid associations are skipped with `warn!` (pre-existing semantics).
    async fn apply_import_payload(
        &self,
        concepts: &[Concept],
        associations: &[(String, String, f32)],
    ) -> Result<Vec<(String, String, f32)>> {
        for concept in concepts {
            self.validate_concept(concept)?;
        }
        let ns = self.namespace().await;

        {
            let mut sing = self.singularity.write().await;
            for concept in concepts {
                sing.inject(&ns, concept.clone())?;
            }
        }

        let mut valid_associations = Vec::with_capacity(associations.len());
        {
            let mut sing = self.singularity.write().await;
            for (from, to, strength) in associations {
                match sing.associate(&ns, from, to, *strength) {
                    Ok(()) => valid_associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import"
                        );
                    }
                }
            }
        }
        Ok(valid_associations)
    }

    async fn clear_for_import_replace(&self) -> Result<()> {
        let ns = self.namespace().await;
        {
            let mut sing = self.singularity.write().await;
            sing.clear(&ns);
        }
        if let Some(ref persistence) = self.persistence {
            persistence.clear_namespace(&ns).await?;
        }
        Ok(())
    }

    async fn persist_import(
        &self,
        concepts: &[Concept],
        associations: &[(String, String, f32)],
    ) -> Result<()> {
        if let Some(persistence) = &self.persistence {
            let ns = self.namespace().await;
            persistence.save_concepts(&ns, concepts).await?;
            persistence.save_associations(&ns, associations).await?;
        }
        Ok(())
    }

    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = self
            .secure_read_file(&validated_path, MAX_IMPORT_SIZE)
            .await?;
        let payload: ExportPayload = serde_json::from_slice(&bytes)?;
        let concepts: Vec<Concept> = payload
            .concepts
            .iter()
            .cloned()
            .map(export_to_concept)
            .collect();

        if !merge {
            self.clear_for_import_replace().await?;
        }
        let valid_associations = self
            .apply_import_payload(&concepts, &payload.associations)
            .await?;
        self.persist_import(&concepts, &valid_associations).await?;
        Ok(concepts.len())
    }

    /// Export memory state to binary file.
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            let json_payload = ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing
                    .all_concepts(&ns)
                    .into_iter()
                    .map(concept_to_export)
                    .collect(),
                associations: sing.all_associations(&ns),
            };
            let res = BinaryExportPayload::from(json_payload);
            drop(sing);
            res
        };

        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let data = options.serialize(&payload).map_err(|e| {
            csm_core_lib::error::MemoryError::Serialization(serde_json::Error::io(
                std::io::Error::other(e.to_string()),
            ))
        })?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from binary file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = self
            .secure_read_file(&validated_path, MAX_IMPORT_SIZE)
            .await?;
        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let binary_payload: BinaryExportPayload = options.deserialize(&bytes).map_err(|e| {
            csm_core_lib::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("bincode deserialization failed: {e}"),
            }
        })?;
        let payload = binary_payload.to_export_payload().map_err(|e| {
            csm_core_lib::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("failed to convert binary payload: {e}"),
            }
        })?;
        let concepts: Vec<Concept> = payload
            .concepts
            .iter()
            .cloned()
            .map(export_to_concept)
            .collect();
        if !merge {
            self.clear_for_import_replace().await?;
        }
        let valid_associations = self
            .apply_import_payload(&concepts, &payload.associations)
            .await?;
        self.persist_import(&concepts, &valid_associations).await?;
        Ok(concepts.len())
    }

    /// Create database backup (SQLite only).
    #[instrument(err, skip(self), fields(path))]
    pub async fn backup(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        if let Some(ref persistence) = self.persistence {
            persistence
                .backup(validated_path.to_str().unwrap_or(path))
                .await?;
        }
        Ok(())
    }

    /// Restore from database backup (SQLite only).
    #[instrument(err, skip(self), fields(path))]
    pub async fn restore(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        if let Some(ref persistence) = self.persistence {
            persistence
                .restore(validated_path.to_str().unwrap_or(path))
                .await?;
            self.load_replace().await?;
        }
        Ok(())
    }

    /// Get version history for a concept.
    #[instrument(err, skip(self), fields(id, limit))]
    pub async fn concept_history(
        &self,
        id: &str,
        mut limit: usize,
    ) -> Result<Vec<crate::singularity::ConceptVersion>> {
        if limit > MAX_HISTORY_LIMIT {
            limit = MAX_HISTORY_LIMIT;
        }
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace().await;
            return persistence.get_concept_history(&ns, id, limit).await;
        }
        Ok(Vec::new())
    }
}
