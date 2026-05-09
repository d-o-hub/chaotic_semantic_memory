#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use crate::error::Result;
use crate::hyperdim::Hypervector;
use crate::export_payload::{BinaryExportPayload, ExportPayload, unix_now_secs};
use crate::framework::ChaoticSemanticFramework;
use crate::framework_events::MemoryEvent;
use crate::framework_validation::validate_path;
use crate::hyperdim::HVec10240;
use crate::singularity::ConceptBuilder;
use bincode::Options;
use std::sync::Arc;
use tokio::fs;
use tracing::{instrument, warn};

const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB default
const MAX_HISTORY_LIMIT: usize = 1000;

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    /// Batch inject multiple concepts into memory.
    // Singularity write lock needed for batch inject
    #[instrument(err, skip(self, concepts))]
    pub async fn inject_concepts(&self, concepts: &[(String, H)]) -> Result<()> {
        self.validate_batch_size(concepts.len())?;

        if concepts.is_empty() {
            return Ok(());
        }

        let mut to_save = Vec::with_capacity(concepts.len());
        {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            for (id, vector) in concepts {
                Self::validate_concept_id(id)?;
                let concept = ConceptBuilder::new(id.clone())
                    .with_vector(*vector)
                    .build()?;
                sing.inject(&ns, concept.clone())?;
                to_save.push(concept);
            }
            drop(sing);
        }

        if let Some(ref persistence) = self.persistence {
            let p_start = std::time::Instant::now();
            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &to_save).await?;
            self.metrics.observe_persist_latency_ms(
                u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                "save",
            );
        }

        self.metrics.inc_concepts_injected(to_save.len() as u64);
        Ok(())
    }

    /// Batch create associations between concepts.
    #[instrument(err, skip(self, associations))]
    pub async fn associate_many(&self, associations: &[(String, String, f32)]) -> Result<()> {
        self.validate_batch_size(associations.len())?;

        if associations.is_empty() {
            return Ok(());
        }

        {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            for (from, to, strength) in associations {
                Self::validate_concept_id(from)?;
                Self::validate_concept_id(to)?;
                Self::validate_association_strength(*strength)?;
                sing.associate(&ns, from, to, *strength)?;
            }
        }

        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            persistence.save_associations(&ns, associations).await?;
        }

        self.metrics
            .inc_associations_created(associations.len() as u64);
        Ok(())
    }

    /// Batch similarity queries without caching.
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Vec<(String, f32)>>> {
        self.validate_top_k(top_k)?;
        self.validate_batch_size(queries.len())?;
        let out = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            queries
                .iter()
                .map(|q| sing.find_similar(&ns, q, top_k))
                .collect()
        };
        Ok(out)
    }

    /// Batch similarity queries with LRU caching.
    #[allow(clippy::type_complexity)]
    #[instrument(err, skip(self, queries))]
    pub async fn probe_batch_cached(
        &self,
        queries: &[HVec10240],
        top_k: usize,
    ) -> Result<Vec<Arc<[(String, f32)]>>> {
        self.validate_top_k(top_k)?;
        self.validate_batch_size(queries.len())?;
        let out = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            queries
                .iter()
                .map(|q| sing.find_similar_cached(&ns, q, top_k))
                .collect()
        };
        Ok(out)
    }

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
                concepts: sing.all_concepts(&ns),
                associations: sing.all_associations(&ns),
            }
        };
        let data = serde_json::to_vec_pretty(&payload)?;
        fs::write(validated_path, data).await?;
        Ok(())
    }
    /// Import memory state from JSON file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_json(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;
        // MAX_IMPORT_SIZE fits in usize on 64-bit
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
                let ns = self.namespace.read().await;
                sing.clear(&ns);
            }
            if let Some(ref persistence) = self.persistence {
                let ns = self.namespace.read().await;
                persistence.clear_namespace(&ns).await?;
            }
        }

        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(&ns, concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(&ns, from, to, *strength) {
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
            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &payload.concepts).await?;
            persistence
                .save_associations(&ns, &valid_associations)
                .await?;
        }
        Ok(payload.concepts.len())
    }
    /// Export memory state to binary file.
    // Singularity read lock needed for binary export
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
                concepts: sing.all_concepts(&ns),
                associations: sing.all_associations(&ns),
            };
            let res = BinaryExportPayload::from(json_payload);
            drop(sing);
            res
        };

        let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
        let data = options.serialize(&payload).map_err(|e| {
            crate::error::MemoryError::Serialization(serde_json::Error::io(std::io::Error::other(
                e.to_string(),
            )))
        })?;
        fs::write(validated_path, data).await?;
        Ok(())
    }

    /// Import memory state from binary file.
    #[instrument(err, skip(self), fields(path, merge))]
    pub async fn import_binary(&self, path: &str, merge: bool) -> Result<usize> {
        let validated_path = validate_path(path)?;
        let bytes = fs::read(validated_path).await?;

        // MAX_IMPORT_SIZE fits in usize on 64-bit
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
        let binary_payload: BinaryExportPayload =
            options
                .deserialize(&bytes)
                .map_err(|e| crate::error::MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("bincode deserialization failed: {e}"),
                })?;
        // Convert to regular payload
        let payload = binary_payload.to_export_payload().map_err(|e| {
            crate::error::MemoryError::InvalidInput {
                field: "import_data".to_string(),
                reason: format!("failed to convert binary payload: {e}"),
            }
        })?;
        if !merge {
            {
                let mut sing = self.singularity.write().await;
                let ns = self.namespace.read().await;
                sing.clear(&ns);
            }
            if let Some(ref persistence) = self.persistence {
                let ns = self.namespace.read().await;
                persistence.clear_namespace(&ns).await?;
            }
        }
        // Acquire write lock, inject concepts + build associations list, then release
        let valid_associations = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            let mut associations = Vec::with_capacity(payload.associations.len());
            for concept in &payload.concepts {
                self.validate_concept(concept)?;
                sing.inject(&ns, concept.clone())?;
            }
            for (from, to, strength) in &payload.associations {
                match sing.associate(&ns, from, to, *strength) {
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
            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &payload.concepts).await?;
            persistence
                .save_associations(&ns, &valid_associations)
                .await?;
        }

        Ok(payload.concepts.len())
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
    ) -> Result<Vec<crate::persistence::ConceptVersion>> {
        if limit > MAX_HISTORY_LIMIT {
            limit = MAX_HISTORY_LIMIT;
        }
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            return persistence.get_concept_history(&ns, id, limit).await;
        }
        Ok(Vec::new())
    }

    /// Update a concept's vector.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        let concept = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.update(&ns, id, vector)?;
            sing.get(&ns, id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            let ns = self.namespace.read().await;
            persistence.save_concept(&ns, &concept).await?;
        }
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        });
        Ok(())
    }

    /// Update a concept's metadata.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_metadata(
        &self,
        id: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let concept = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.update_metadata(&ns, id, metadata)?;
            sing.get(&ns, id).cloned()
        };

        if let (Some(concept), Some(persistence)) = (concept, &self.persistence) {
            let ns = self.namespace.read().await;
            persistence.save_concept(&ns, &concept).await?;
        }
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        });
        Ok(())
    }

    /// Remove an association between two concepts.
    #[instrument(err, skip(self), fields(from, to))]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.disassociate(&ns, from, to)?;
        }

        if let Some(persistence) = &self.persistence {
            let ns = self.namespace.read().await;
            persistence.delete_association(&ns, from, to).await?;
        }
        self.emit_event(MemoryEvent::Disassociated {
            from: from.to_string(),
            to: to.to_string(),
        });
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    #[instrument(err, skip(self), fields(id))]
    pub async fn clear_associations(&self, id: &str) -> Result<()> {
        {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.clear_associations(&ns, id)?;
        }

        if let Some(persistence) = &self.persistence {
            let ns = self.namespace.read().await;
            persistence.clear_concept_associations(&ns, id).await?;
        }
        Ok(())
    }

    /// Clear the similarity query cache.
    pub async fn clear_similarity_cache(&self) {
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.invalidate_cache(&ns);
    }

    /// Bundle multiple concepts into a single hypervector (strict version).
    pub async fn bundle_concepts_strict(&self, ids: &[String]) -> Result<HVec10240> {
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.bundle_concepts_strict(&ns, ids)
    }
}
