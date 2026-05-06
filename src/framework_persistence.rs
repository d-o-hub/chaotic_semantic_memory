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
            // ADR-0068: Persist ANN index state
            {
                let sing = self.singularity.read().await;
                if let Some(ns_state) = sing.get_namespace(&self.namespace) {
                    if let Ok(index_data) = ns_state.index.serialize() {
                        if !index_data.is_empty() {
                            persistence
                                .save_index(&self.namespace, "main", &index_data)
                                .await?;
                        }
                    }
                }
            }

            persistence.checkpoint().await?;
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
        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_concepts(&self.namespace).await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                let links = persistence
                    .load_associations(&self.namespace, &concept.id)
                    .await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                sing.clear(&self.namespace);
                for concept in concepts {
                    sing.inject(&self.namespace, concept)?;
                }
                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&self.namespace, &from_id, &to_id, strength)
                    {
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
            if let Ok(Some(index_data)) = persistence.load_index(&self.namespace, "main").await {
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&self.namespace);
                    let _ = ns_state.index.deserialize(&index_data);
                }
            } else {
                // Fallback: rebuild index from concepts
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&self.namespace);
                    let concepts_map = ns_state.concepts.clone();
                    let _ = ns_state.index.rebuild(&concepts_map);
                }
            }
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
            let concepts = persistence.load_all_concepts(&self.namespace).await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            {
                let mut sing = self.singularity.write().await;
                for concept in &concepts {
                    if sing.get(&self.namespace, &concept.id).is_some() {
                        warn!(
                            concept_id = %concept.id,
                            "skipping persisted concept during load_merge because id already exists in memory"
                        );
                        continue;
                    }
                    sing.inject(&self.namespace, (*concept).clone())?;
                }
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept in &concepts {
                let links = persistence
                    .load_associations(&self.namespace, &concept.id)
                    .await?;
                for (to_id, strength) in links {
                    all_associations.push((concept.id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                for (from_id, to_id, strength) in all_associations {
                    if let Err(error) = sing.associate(&self.namespace, &from_id, &to_id, strength)
                    {
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
            if let Ok(Some(index_data)) = persistence.load_index(&self.namespace, "main").await {
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&self.namespace);
                    let _ = ns_state.index.deserialize(&index_data);
                }
            } else {
                // Fallback: rebuild index from concepts
                {
                    let mut sing = self.singularity.write().await;
                    let ns_state = sing.get_namespace_mut(&self.namespace);
                    let concepts_map = ns_state.concepts.clone();
                    let _ = ns_state.index.rebuild(&concepts_map);
                }
            }
        }
        Ok(())
    }
}
