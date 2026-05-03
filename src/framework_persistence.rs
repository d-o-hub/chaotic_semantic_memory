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
            let data = self.singularity.read().await.index.serialize();
            if let Ok(index_data) = data {
                if !index_data.is_empty() {
                    persistence.save_index("main", &index_data).await?;
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
    #[tracing::instrument(err, skip(self))]
    pub async fn load_replace(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_concepts().await?;

            let mut concept_ids = Vec::with_capacity(concepts.len());
            for concept in &concepts {
                self.validate_concept(concept)?;
                concept_ids.push(concept.id.clone());
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept_id in &concept_ids {
                let links = persistence.load_associations(concept_id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept_id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
                sing.clear();
                for concept in concepts {
                    sing.inject(concept)?;
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
                if let Some(ref persistence) = self.persistence {
                    if let Ok(Some(index_data)) = persistence.load_index("main").await {
                        let _ = sing.index.deserialize(&index_data);
                    } else {
                        // Fallback: rebuild index from concepts
                        let concepts = sing.concepts.clone();
                        let _ = sing.index.rebuild(&concepts);
                    }
                }
            }
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
            let concepts = persistence.load_all_concepts().await?;

            for concept in &concepts {
                self.validate_concept(concept)?;
            }

            let concept_ids: Vec<String> = concepts.iter().map(|c| c.id.clone()).collect();

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
            }

            let mut all_associations: Vec<(String, String, f32)> = Vec::new();
            for concept_id in &concept_ids {
                let links = persistence.load_associations(concept_id).await?;
                for (to_id, strength) in links {
                    all_associations.push((concept_id.clone(), to_id, strength));
                }
            }

            {
                let mut sing = self.singularity.write().await;
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

                // ADR-0068: Load ANN index state
                if let Some(ref persistence) = self.persistence {
                    if let Ok(Some(index_data)) = persistence.load_index("main").await {
                        let _ = sing.index.deserialize(&index_data);
                    } else {
                        // Fallback: rebuild index from concepts
                        let concepts = sing.concepts.clone();
                        let _ = sing.index.rebuild(&concepts);
                    }
                }
            }
        }
        Ok(())
    }
}
