//! Framework persistence operations.
//!
//! Extracted from framework.rs to satisfy the 500 LOC gate.

use tracing::warn;

use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;

impl ChaoticSemanticFramework {
    /// Persist all data to storage
    #[tracing::instrument(err, skip(self))]
    pub async fn persist(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let p_start = std::time::Instant::now();
            // ADR-0068: Persist ANN index state
            // #13: Propagate serialization errors
            let index_data = self.singularity.read().await.index.serialize()?;
            if !index_data.is_empty() {
                persistence.save_index("main", &index_data).await?;
            }

            persistence.checkpoint().await?;
            #[allow(clippy::cast_possible_truncation)]
            self.metrics
                .observe_persist_latency_ms(p_start.elapsed().as_millis() as u64, "persist");
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
    #[tracing::instrument(err, skip(self))]
    pub async fn load_replace(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let p_start = std::time::Instant::now();
            // #12: Fetch data outside of singularity lock
            let concepts = persistence.load_all_concepts().await?;

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                self.validate_concept(concept)?;
                let links = persistence.load_associations(&concept.id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            // #12: Load index outside of singularity lock
            let index_data = persistence.load_index("main").await.ok().flatten();

            {
                let mut sing = self.singularity.write().await;
                sing.clear();

                // #11: Avoid cloning the entire concepts map by moving/processing them directly
                for concept in concepts {
                    let concept_id = concept.id.clone();
                    let concept_vector = concept.vector;

                    // Manual inject to avoid redundant index inserts if we're about to deserialize
                    if let Some(&idx) = sing.id_to_index.get(&concept_id) {
                        sing.concept_vectors[idx] = concept_vector;
                    } else {
                        let idx = sing.concept_indices.len();
                        sing.id_to_index.insert(concept_id.clone(), idx);
                        sing.concept_indices.push(concept_id.clone());
                        sing.concept_vectors.push(concept_vector);
                    }
                    sing.concepts.insert(concept_id, concept);
                }

                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_replace"
                        );
                    }
                }

                // ADR-0068: Load ANN index state
                // #2, #3: Prefer deserialize over rebuild if data is fresh
                if let Some(data) = index_data {
                    let _ = sing.index.deserialize(&data);
                } else {
                    // Fallback: rebuild index from concepts
                    // We must clone because rebuild takes &HashMap but we have &mut Singularity
                    let concepts = sing.concepts.clone();
                    let _ = sing.index.rebuild(&concepts);
                }
                sing.invalidate_cache();
            }
            #[allow(clippy::cast_possible_truncation)]
            self.metrics
                .observe_persist_latency_ms(p_start.elapsed().as_millis() as u64, "load");
        }
        Ok(())
    }

    /// Load and merge persisted state into in-memory state.
    ///
    /// Preserves existing state, adds persisted state on top.
    /// See also: [`load_replace`](Self::load_replace) for replacement semantics.
    #[tracing::instrument(err, skip(self))]
    pub async fn load_merge(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            // #12: Fetch data outside of singularity lock
            let concepts = persistence.load_all_concepts().await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                let links = persistence.load_associations(&concept.id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                for concept in concepts {
                    if sing.get(&concept.id).is_some() {
                        warn!(
                            concept_id = %concept.id,
                            "skipping persisted concept during load_merge because id already exists in memory"
                        );
                        continue;
                    }
                    sing.inject(concept)?;
                }

                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&from_id, &to_id, strength) {
                        warn!(
                            from_id = %from_id,
                            to_id = %to_id,
                            strength,
                            error = %error,
                            "skipping invalid association during load_merge"
                        );
                    }
                }

                // #4: load_merge should rebuild/merge instead of blind deserialize
                // We just injected new concepts into the index via sing.inject(),
                // so the index is already updated with merged concepts.
                // Rebuilding ensures optimal structure if many concepts were merged.
                let concepts = sing.concepts.clone();
                let _ = sing.index.rebuild(&concepts);
            }
        }
        Ok(())
    }
}
