#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use crate::error::Result;
use crate::export_payload::unix_now_secs;
use crate::framework_validation::validate_path;
use crate::framework::ChaoticSemanticFramework;
use crate::framework_events::MemoryEvent;
use crate::hyperdim::HVec10240;
use crate::singularity::ConceptBuilder;
use std::sync::Arc;
use tracing::instrument;

const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB default
const MAX_HISTORY_LIMIT: usize = 1000;

impl ChaoticSemanticFramework {
    /// Batch inject multiple concepts into memory.
    // Singularity write lock needed for batch inject
    #[instrument(err, skip(self, concepts))]
    pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
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
            #[cfg(not(target_arch = "wasm32"))]
            let p_start = std::time::Instant::now();
            #[cfg(target_arch = "wasm32")]
            let p_start = 0.0; // js_sys::Date not imported here, and p_start not used for WASM

            let ns = self.namespace.read().await;
            persistence.save_concepts(&ns, &to_save).await?;

            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = 0; // Persistence always None on WASM, but keep for completeness

            self.metrics.observe_persist_latency_ms(elapsed_ms, "save");
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
            let ns = self.namespace.read().await;
            return persistence.get_concept_history(&ns, id, limit).await;
        }
        Ok(Vec::new())
    }

    /// Update a concept's vector.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        Self::validate_concept_id(id)?;
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
        })
        .await;
        Ok(())
    }

    /// Update a concept's metadata.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_metadata(
        &self,
        id: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Self::validate_concept_id(id)?;
        Self::validate_metadata_bytes(&metadata, self.config.max_metadata_bytes)?;
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
        })
        .await;
        Ok(())
    }

    /// Remove an association between two concepts.
    #[instrument(err, skip(self), fields(from, to))]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
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
        })
        .await;
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    #[instrument(err, skip(self), fields(id))]
    pub async fn clear_associations(&self, id: &str) -> Result<()> {
        Self::validate_concept_id(id)?;
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
        self.validate_batch_size(ids.len())?;
        let sing = self.singularity.read().await;
        let ns = self.namespace.read().await;
        sing.bundle_concepts_strict(&ns, ids)
    }
}
