#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
use crate::concept_builder::ConceptBuilder;
use crate::export_payload::unix_now_secs;
use crate::framework::ChaoticSemanticFramework;
use crate::framework_events::MemoryEvent;
use csm_core_lib::error::{MemoryError, Result};
use csm_core_lib::hyperdim::HVec10240;
use std::sync::Arc;
use tracing::instrument;

// Singularity is Send + Sync for Rayon probe/inject construction (#525 / probe_batch).
const _: () = {
    const fn assert_sync_send<T: Sync + Send>() {}
    assert_sync_send::<crate::singularity::Singularity>();
};

impl ChaoticSemanticFramework {
    /// Batch inject: build concepts (optionally parallel) then durable commit.
    /// Construction runs **before** any write lock (`durable_inject_concepts`).
    #[instrument(err, skip(self, concepts))]
    pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
        self.validate_batch_size(concepts.len())?;
        if concepts.is_empty() {
            return Ok(());
        }

        // #525: parallel CPU-bound ConceptBuilder work outside the write lock.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let to_save: Vec<_> = {
            use rayon::prelude::*;
            concepts
                .par_iter()
                .map(|(id, vector)| {
                    Self::validate_concept_id(id)?;
                    ConceptBuilder::new(id.clone()).with_vector(*vector).build()
                })
                .collect::<Result<Vec<_>>>()?
        };
        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let to_save: Vec<_> = {
            let mut out = Vec::with_capacity(concepts.len());
            for (id, vector) in concepts {
                Self::validate_concept_id(id)?;
                out.push(
                    ConceptBuilder::new(id.clone())
                        .with_vector(*vector)
                        .build()?,
                );
            }
            out
        };

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        self.durable_inject_concepts(&to_save).await?;
        if self.persistence.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = 0u64;
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
        for (from, to, strength) in associations {
            Self::validate_concept_id(from)?;
            Self::validate_concept_id(to)?;
            Self::validate_association_strength(*strength)?;
        }

        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;
        {
            let sing = self.singularity.read().await;
            for (from, to, _) in associations {
                if sing.get(&ns, from).is_none() {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: from.clone(),
                    });
                }
                if sing.get(&ns, to).is_none() {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: to.clone(),
                    });
                }
            }
        }

        if let Some(persistence) = &self.persistence {
            persistence.save_associations(&ns, associations).await?;
        }

        let apply_result = {
            let mut sing = self.singularity.write().await;
            associations
                .iter()
                .try_for_each(|(from, to, strength)| sing.associate(&ns, from, to, *strength))
        };
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit association batch failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit association batch failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
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
            let now = crate::singularity::unix_now_secs();

            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            {
                use rayon::prelude::*;
                queries
                    .par_iter()
                    .map(|query| {
                        sing.find_similar_active(&ns, query, top_k, now, None)
                            .as_ref()
                            .to_vec()
                    })
                    .collect()
            }

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            {
                queries
                    .iter()
                    .map(|query| {
                        sing.find_similar_active(&ns, query, top_k, now, None)
                            .as_ref()
                            .to_vec()
                    })
                    .collect()
            }
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
            let now = crate::singularity::unix_now_secs();

            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            {
                use rayon::prelude::*;
                queries
                    .par_iter()
                    .map(|query| sing.find_similar_active(&ns, query, top_k, now, None))
                    .collect()
            }

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            {
                queries
                    .iter()
                    .map(|query| sing.find_similar_active(&ns, query, top_k, now, None))
                    .collect()
            }
        };
        Ok(out)
    }

    /// Update a concept's vector.
    #[instrument(err, skip(self), fields(id))]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        Self::validate_concept_id(id)?;
        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;
        let mut updated = {
            let sing = self.singularity.read().await;
            sing.get(&ns, id)
                .cloned()
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "Concept".to_string(),
                    id: id.to_string(),
                })?
        };
        updated.vector = vector;
        updated.modified_at = unix_now_secs();

        if let Some(persistence) = &self.persistence {
            persistence.save_concept(&ns, &updated).await?;
        }
        let apply_result = self.singularity.write().await.inject(&ns, updated.clone());
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit vector update failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit vector update failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
        }
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: updated.modified_at,
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
        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;
        let mut updated = {
            let sing = self.singularity.read().await;
            sing.get(&ns, id)
                .cloned()
                .ok_or_else(|| MemoryError::NotFound {
                    entity: "Concept".to_string(),
                    id: id.to_string(),
                })?
        };
        updated.metadata = metadata;
        updated.modified_at = unix_now_secs();

        if let Some(persistence) = &self.persistence {
            persistence.save_concept(&ns, &updated).await?;
        }
        let apply_result = self.singularity.write().await.inject(&ns, updated.clone());
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit metadata update failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit metadata update failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
        }
        self.invalidate_text_index(&ns).await;
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: updated.modified_at,
        })
        .await;
        Ok(())
    }

    /// Remove an association between two concepts.
    #[instrument(err, skip(self), fields(from, to))]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_concept_id(from)?;
        Self::validate_concept_id(to)?;
        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;

        if let Some(persistence) = &self.persistence {
            persistence.delete_association(&ns, from, to).await?;
        }
        let apply_result = self.singularity.write().await.disassociate(&ns, from, to);
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit disassociate failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit disassociate failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
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
        let _mutation = self.mutation_coordinator.lock().await;
        let ns = self.namespace().await;

        if let Some(persistence) = &self.persistence {
            persistence.clear_concept_associations(&ns, id).await?;
        }
        let apply_result = self.singularity.write().await.clear_associations(&ns, id);
        if let Err(error) = apply_result {
            if self.persistence.is_some() {
                if let Err(reload) = self.reload_namespace_from_rows(&ns).await {
                    return Err(MemoryError::database(format!(
                        "post-commit association clear failed ({error}); namespace reconciliation failed ({reload})"
                    )));
                }
                return Err(MemoryError::database(format!(
                    "post-commit association clear failed ({error}); namespace was reloaded from durable rows"
                )));
            }
            return Err(error);
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
