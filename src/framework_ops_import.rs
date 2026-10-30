//! Export, import, backup/restore, and concept history operations.
//!
//! Split from `framework_ops.rs` to respect the 500-LOC module ceiling.

#[cfg(test)]
#[path = "framework_ops_import_tests.rs"]
mod import_transaction_tests;
use crate::export_payload::{
    BinaryExportPayload, ExportPayload, concept_to_export, decode_binary_export,
    encode_binary_export, export_to_concept, unix_now_secs,
};
use crate::framework::ChaoticSemanticFramework;
use crate::framework_validation::{MAX_IMPORT_SIZE, validate_path};
use crate::singularity::Concept;
use csm_core_lib::error::{MemoryError, Result};
use tokio::fs;
use tracing::instrument;

const MAX_HISTORY_LIMIT: usize = 1000;

impl ChaoticSemanticFramework {
    /// Export memory state to JSON file.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let ns = self.namespace().await;
        self.ensure_namespace_loaded(&ns).await?;

        let payload = {
            let sing = self.singularity.read().await;
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

    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let result = self.import_json_inner(path, merge).await;
        if result.is_err() {
            #[cfg(feature = "prometheus")]
            crate::observability::prom::record_import_failure("json");
        }
        result
    }

    async fn import_json_inner(&self, path: &str, merge: bool) -> Result<usize> {
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

        self.apply_import_payload(&concepts, &payload.associations, merge)
            .await
    }

    /// Export memory state to a bounded, versioned binary payload.
    #[allow(clippy::significant_drop_tightening)]
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let ns = self.namespace().await;
        self.ensure_namespace_loaded(&ns).await?;

        let payload = {
            let sing = self.singularity.read().await;
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
            BinaryExportPayload::from(json_payload)
        };

        let data = encode_binary_export(&payload)?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from a binary payload.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let result = self.import_binary_inner(path, merge).await;
        if result.is_err() {
            #[cfg(feature = "prometheus")]
            crate::observability::prom::record_import_failure("binary");
        }
        result
    }

    async fn import_binary_inner(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = self
            .secure_read_file(&validated_path, MAX_IMPORT_SIZE)
            .await?;
        let binary_payload = decode_binary_export(&bytes)?;
        let payload =
            binary_payload
                .to_export_payload()
                .map_err(|e| MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("failed to convert binary payload: {e}"),
                })?;
        let concepts: Vec<Concept> = payload
            .concepts
            .iter()
            .cloned()
            .map(export_to_concept)
            .collect();
        self.apply_import_payload(&concepts, &payload.associations, merge)
            .await
    }

    /// Create a database backup (SQLite only).
    #[instrument(err, skip(self), fields(path))]
    pub async fn backup(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let persistence = self.persistence.as_ref().ok_or_else(|| {
            MemoryError::UnsupportedOperation(
                "Persistence is required to create a database backup".to_string(),
            )
        })?;
        persistence
            .backup(validated_path.to_str().unwrap_or(path))
            .await
    }

    /// Restore a database backup and rebuild the current namespace from rows.
    #[instrument(err, skip(self), fields(path))]
    pub async fn restore(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;
        let persistence = self.persistence.as_ref().ok_or_else(|| {
            MemoryError::UnsupportedOperation(
                "Persistence is required to restore a database backup".to_string(),
            )
        })?;
        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;
        persistence
            .restore(validated_path.to_str().unwrap_or(path))
            .await?;
        self.reload_namespace_from_rows(&ns).await
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
