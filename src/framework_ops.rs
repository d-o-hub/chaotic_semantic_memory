use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{instrument, warn};

use crate::error::{MemoryError, Result};
use crate::export_payload::{ExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::HVec10240;
use crate::singularity::ConceptBuilder;
use bincode::Options;

const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB default
const MAX_PATH_LENGTH: usize = 4096;

fn validate_path(path: &str) -> Result<PathBuf> {
    if path.len() > MAX_PATH_LENGTH {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: format!(
                "path exceeds maximum length of {} characters",
                MAX_PATH_LENGTH
            ),
        });
    }

    let path = PathBuf::from(path);

    if path
        .components()
        .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: "path traversal '..' components are not allowed".to_string(),
        });
    }

    if path.is_absolute() {
        let normalized = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Err(MemoryError::InvalidInput {
                    field: "path".to_string(),
                    reason: "absolute path does not exist or cannot be accessed".to_string(),
                });
            }
        };

        let current_dir = std::env::current_dir().map_err(|e| MemoryError::InvalidInput {
            field: "path".to_string(),
            reason: format!("cannot determine current working directory: {}", e),
        })?;

        if !normalized.starts_with(&current_dir) && !normalized.starts_with("/tmp") {
            return Err(MemoryError::InvalidInput {
                field: "path".to_string(),
                reason: "absolute paths must be within current working directory or /tmp"
                    .to_string(),
            });
        }
    }

    Ok(path)
}

impl ChaoticSemanticFramework {
    /// Batch inject multiple concepts into memory.
    ///
    /// Each concept is validated and inserted atomically. If persistence is enabled,
    /// concepts are persisted to the database in a single batch operation.
    #[instrument(err, skip(self, concepts))]
    pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
        if concepts.is_empty() {
            return Ok(());
        }

        let mut to_save = Vec::with_capacity(concepts.len());
        {
            let mut sing = self.singularity.write().await;
            for (id, vector) in concepts {
                Self::validate_concept_id(id)?;
                let concept = ConceptBuilder::new(id.clone())
                    .with_vector(*vector)
                    .build()?;
                sing.inject(concept.clone())?;
                to_save.push(concept);
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concepts(&to_save).await?;
        }

        self.metrics.inc_concepts_injected(to_save.len() as u64);
        Ok(())
    }

    /// Batch create associations between concepts.
    ///
    /// Each association is validated before insertion. If persistence is enabled,
    /// associations are persisted in a single batch operation.
    #[instrument(err, skip(self, associations))]
    pub async fn associate_many(&self, associations: &[(String, String, f32)]) -> Result<()> {
        if associations.is_empty() {
            return Ok(());
        }

        {
            let mut sing = self.singularity.write().await;
            for (from, to, strength) in associations {
                Self::validate_concept_id(from)?;
                Self::validate_concept_id(to)?;
                Self::validate_association_strength(*strength)?;
                sing.associate(from, to, *strength)?;
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_associations(associations).await?;
        }

        self.metrics
            .inc_associations_created(associations.len() as u64);
        Ok(())
    }

    /// Batch similarity queries without caching.
    ///
    /// Returns similarity results for each query vector. Results are not cached.
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Vec<(String, f32)>>> {
        self.validate_top_k(top_k)?;
        let sing = self.singularity.read().await;
        let mut out = Vec::with_capacity(queries.len());
        for query in queries {
            out.push(sing.find_similar(query, top_k));
        }
        Ok(out)
    }

    /// Batch similarity queries with LRU caching.
    ///
    /// Results are cached and reused for identical queries. Returns Arc references
    /// to avoid cloning large result sets.
    #[allow(clippy::type_complexity)]
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch_cached(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Arc<[(String, f32)]>>> {
        self.validate_top_k(top_k)?;
        let sing = self.singularity.read().await;
        let mut out = Vec::with_capacity(queries.len());
        for query in queries {
            out.push(sing.find_similar_cached(query, top_k));
        }
        Ok(out)
    }

    /// Export memory state to JSON file.
    ///
    /// Writes all concepts and associations to the specified path in JSON format.
    /// Useful for backups, debugging, and interoperability.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_json(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(),
                associations: sing.all_associations(),
            }
        };

        let data = serde_json::to_vec_pretty(&payload)?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from JSON file.
    ///
    /// If `merge` is false, clears existing state before importing.
    /// Returns the number of concepts imported.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;
        if bytes.len() > MAX_IMPORT_SIZE as usize {
            return Err(crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!(
                    "JSON import data size {} exceeds maximum allowed size {}",
                    bytes.len(),
                    MAX_IMPORT_SIZE
                ),
            });
        }
        let payload: ExportPayload = serde_json::from_slice(&bytes)?;

        if !merge {
            {
                let mut sing = self.singularity.write().await;
                sing.clear();
            }
            if let Some(ref persistence) = self.persistence {
                persistence.clear_all().await?;
            }
        }

        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(from, to, *strength) {
                    Ok(()) => associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import_json"
                        );
                    }
                }
            }
            associations
        }; // Lock released here

        // Persist concepts and associations (no lock needed)
        if let Some(ref persistence) = self.persistence {
            persistence.save_concepts(&payload.concepts).await?;
            persistence.save_associations(&valid_associations).await?;
        }

        Ok(payload.concepts.len())
    }

    /// Export memory state to binary file.
    ///
    /// Uses bincode for compact serialization. More efficient than JSON for
    /// large datasets.
    #[instrument(err, skip(self), fields(path))]
    pub async fn export_binary(&self, path: &str) -> Result<()> {
        let validated_path = validate_path(path)?;

        let payload = {
            let sing = self.singularity.read().await;
            ExportPayload {
                version: env!("CARGO_PKG_VERSION").to_string(),
                exported_at: unix_now_secs(),
                concepts: sing.all_concepts(),
                associations: sing.all_associations(),
            }
        };

        let data = bincode::serialize(&payload).map_err(|e| {
            crate::error::MemoryError::Serialization(serde_json::Error::io(std::io::Error::other(
                e.to_string(),
            )))
        })?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from binary file.
    ///
    /// If `merge` is false, clears existing state before importing.
    /// Returns the number of concepts imported.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;

        if bytes.len() > MAX_IMPORT_SIZE as usize {
            return Err(crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!(
                    "import data size {} exceeds maximum allowed size {}",
                    bytes.len(),
                    MAX_IMPORT_SIZE
                ),
            });
        }

        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let payload: ExportPayload =
            options
                .deserialize(&bytes)
                .map_err(|e| crate::error::MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("bincode deserialization failed: {}", e),
                })?;

        if !merge {
            {
                let mut sing = self.singularity.write().await;
                sing.clear();
            }
            if let Some(ref persistence) = self.persistence {
                persistence.clear_all().await?;
            }
        }

        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(from, to, *strength) {
                    Ok(()) => associations.push((from.clone(), to.clone(), *strength)),
                    Err(error) => {
                        warn!(
                            from_id = %from,
                            to_id = %to,
                            strength = *strength,
                            error = %error,
                            "skipping invalid association during import_binary"
                        );
                    }
                }
            }
            associations
        }; // Lock released here

        // Persist concepts and associations (no lock needed)
        if let Some(ref persistence) = self.persistence {
            persistence.save_concepts(&payload.concepts).await?;
            persistence.save_associations(&valid_associations).await?;
        }

        Ok(payload.concepts.len())
    }

    /// Create database backup (SQLite only).
    ///
    /// Creates a copy of the database file. Only works with local SQLite databases.
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
    ///
    /// Replaces the current database with the backup and reloads memory state.
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
    ///
    /// Returns up to `limit` previous versions of the concept, ordered by
    /// version number descending. Returns empty vec if persistence is disabled.
    #[instrument(err, skip(self), fields(id, limit))]
    pub async fn concept_history(
        &self,
        id: &str,
        limit: usize,
    ) -> Result<Vec<crate::persistence::ConceptVersion>> {
        if let Some(ref persistence) = self.persistence {
            return persistence.get_concept_history(id, limit).await;
        }
        Ok(Vec::new())
    }

    /// Update a concept's vector.
    ///
    /// Updates the vector in memory and persists the change if persistence is enabled.
    /// Records a new version in the version history.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        let concept = {
            let mut sing = self.singularity.write().await;
            sing.update(id, vector)?;
            sing.get(id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            persistence.save_concept(&concept).await?;
        }
        Ok(())
    }

    /// Update a concept's metadata.
    ///
    /// Updates the metadata in memory and persists the change if persistence is enabled.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_metadata(
        &self,
        id: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let concept = {
            let mut sing = self.singularity.write().await;
            sing.update_metadata(id, metadata)?;
            sing.get(id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            persistence.save_concept(&concept).await?;
        }
        Ok(())
    }

    /// Remove an association between two concepts.
    ///
    /// Removes the association from memory and persists the change if persistence is enabled.
    #[instrument(err, skip(self), fields(from, to))]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        {
            let mut sing = self.singularity.write().await;
            sing.disassociate(from, to)?;
        }

        if let Some(persistence) = &self.persistence {
            persistence.delete_association(from, to).await?;
        }
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    ///
    /// Removes all associations from the given concept in memory and persists
    /// the change if persistence is enabled.
    #[instrument(err, skip(self), fields(id))]
    pub async fn clear_associations(&self, id: &str) -> Result<()> {
        {
            let mut sing = self.singularity.write().await;
            sing.clear_associations(id)?;
        }

        if let Some(persistence) = &self.persistence {
            persistence.clear_concept_associations(id).await?;
        }
        Ok(())
    }

    /// Clear the similarity query cache.
    ///
    /// Useful when you want to ensure fresh similarity results.
    pub async fn clear_similarity_cache(&self) {
        let sing = self.singularity.read().await;
        sing.clear_similarity_cache();
    }

    /// Bundle multiple concepts into a single hypervector (strict version).
    ///
    /// Returns `NotFound` error if any concept ID is missing.
    pub async fn bundle_concepts_strict(&self, ids: &[String]) -> Result<HVec10240> {
        let sing = self.singularity.read().await;
        sing.bundle_concepts_strict(ids)
    }
}
