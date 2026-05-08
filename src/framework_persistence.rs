//! Framework persistence operations.
//!
//! Extracted from framework.rs to satisfy the 500 LOC gate.

use tracing::warn;

use crate::error::Result;
use crate::hyperdim::Hypervector;
use crate::framework::ChaoticSemanticFramework;

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    /// Persist all data to storage
    #[tracing::instrument(err, skip(self))]
    pub async fn persist(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let p_start = std::time::Instant::now();
            // ADR-0068: Persist ANN index state
            {
                let sing = self.singularity.read().await;
                let ns = self.namespace.read().await;
                if let Some(ns_state) = sing.get_namespace(&ns) {
                    if let Ok(index_data) = ns_state.index.serialize() {
                        if !index_data.is_empty() {
                            persistence.save_index(&ns, "main", &index_data).await?;
                        }
                    }
                }
            }

            persistence.checkpoint().await?;
            self.metrics.observe_persist_latency_ms(
                u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                "persist",
            );
        }
        Ok(())
    }

    /// Verify persistence connectivity.
    #[tracing::instrument(err, skip(self))]
    pub async fn persistence_health_check(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            persistence.health_check().await?;
        }
        Ok(())
    }

    /// Load and replace all in-memory state from persistence.
    ///
    /// Clears existing state, loads persisted state. Use for fresh starts.
    /// See also: [`load_merge`](Self::load_merge) for additive semantics.
    #[allow(clippy::significant_drop_tightening)] // Lock held for concept injection and index rebuild
    #[tracing::instrument(err, skip(self))]
    pub async fn load_replace(&self) -> Result<()> {
        let p_start = std::time::Instant::now();
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            let concepts = persistence.load_all_concepts(&ns).await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                let links = persistence.load_associations(&ns, &concept.id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                sing.clear(&ns);
                for concept in concepts {
                    sing.inject(&ns, concept)?;
                }

                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&ns, &from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_replace"
                        );
                    }
                }
            }

            // ADR-0068: Load ANN index state
            // Prefer deserialize over rebuild if data is fresh.
            // Propagate errors instead of silently ignoring (fixes test regression).
            if let Ok(Some(index_data)) = persistence.load_index(&ns, "main").await {
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&ns);
                    ns_state.index.deserialize(&index_data)?;
                }
            } else {
                // Fallback: rebuild index from concepts
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&ns);
                    let concepts_map = ns_state.concepts.clone();
                    ns_state.index.rebuild(&concepts_map)?;
                }
            }
            self.metrics.observe_persist_latency_ms(
                u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX),
                "load",
            );
        }
        Ok(())
    }

    /// Load and merge persisted state into in-memory state.
    ///
    /// Preserves existing state, adds persisted state on top.
    /// See also: [`load_replace`](Self::load_replace) for replacement semantics.
    #[allow(clippy::significant_drop_tightening)] // Lock held for concept injection and index rebuild
    #[tracing::instrument(err, skip(self))]
    pub async fn load_merge(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let ns = self.namespace.read().await;
            let concepts = persistence.load_all_concepts(&ns).await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            {
                let mut sing = self.singularity.write().await;
                for concept in &concepts {
                    if sing.get(&ns, &concept.id).is_some() {
                        warn!(
                            concept_id = %concept.id,
                            "skipping persisted concept during load_merge because id already exists in memory"
                        );
                        continue;
                    }
                    sing.inject(&ns, (*concept).clone())?;
                }
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                let links = persistence.load_associations(&ns, &concept.id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&ns, &from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_merge"
                        );
                    }
                }
            }

            // ADR-0068: Load ANN index state
            // We just injected new concepts into the index via sing.inject(),
            // so the index is already updated with merged concepts.
            // Rebuilding ensures optimal structure if many concepts were merged.
            // Propagate errors instead of silently ignoring.
            if let Ok(Some(index_data)) = persistence.load_index(&ns, "main").await {
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&ns);
                    ns_state.index.deserialize(&index_data)?;
                }
            } else {
                // Fallback: rebuild index from concepts
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&ns);
                    let concepts_map = ns_state.concepts.clone();
                    ns_state.index.rebuild(&concepts_map)?;
                }
            }
        }
        Ok(())
    }
}
