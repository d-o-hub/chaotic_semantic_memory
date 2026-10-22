//! Transactional import staging shared by the file, byte and WASM entry points.
//!
//! Split from `framework_ops_import.rs` because that module is native-only
//! (`tokio::fs`), while `WasmFramework::import_from_bytes` must reach the same
//! validate-then-install path: a second, weaker wasm-only import path is how the
//! two would drift. Nothing here touches the filesystem.

use crate::framework::ChaoticSemanticFramework;
use crate::singularity::{Concept, Singularity};
use csm_core_lib::error::{MemoryError, Result};
use std::collections::HashSet;

impl ChaoticSemanticFramework {
    /// Validate, stage, and atomically install an import payload.
    pub(crate) async fn apply_import_payload(
        &self,
        concepts: &[Concept],
        associations: &[(String, String, f32)],
        merge: bool,
    ) -> Result<usize> {
        loop {
            let ns = self.namespace().await;
            if merge {
                self.ensure_namespace_loaded(&ns).await?;
            }
            let _mutation = self.mutation_coordinator.lock().await;
            if self.namespace().await != ns {
                continue;
            }

            self.validate_import_payload(&ns, concepts, associations, merge)
                .await?;
            let mut staged = self
                .prepare_import_state(&ns, concepts, associations, merge)
                .await?;

            if let Some(persistence) = &self.persistence {
                if merge {
                    persistence
                        .merge_namespace_transactional(&ns, concepts, associations)
                        .await?;
                } else {
                    persistence
                        .replace_namespace_transactional(&ns, concepts, associations)
                        .await?;
                }
            }

            let replacement = staged.namespaces.remove(&ns);
            let mut sing = self.singularity.write().await;
            if let Some(state) = replacement {
                sing.namespaces.insert(ns.clone(), state);
            } else {
                sing.namespaces.remove(&ns);
            }
            drop(sing);
            self.invalidate_text_index(&ns).await;
            return Ok(concepts.len());
        }
    }

    async fn validate_import_payload(
        &self,
        ns: &str,
        concepts: &[Concept],
        associations: &[(String, String, f32)],
        merge: bool,
    ) -> Result<()> {
        let mut imported_ids = HashSet::with_capacity(concepts.len());
        for concept in concepts {
            self.validate_concept(concept)?;
            if !imported_ids.insert(concept.id.as_str()) {
                return Err(MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("duplicate concept id: {}", concept.id),
                });
            }
        }

        let mut seen_edges = HashSet::with_capacity(associations.len());
        let sing = self.singularity.read().await;
        for (from, to, strength) in associations {
            Self::validate_concept_id(from)?;
            Self::validate_concept_id(to)?;
            Self::validate_association_strength(*strength)?;
            if !seen_edges.insert((from.as_str(), to.as_str())) {
                return Err(MemoryError::InvalidInput {
                    field: "import_data".to_string(),
                    reason: format!("duplicate association: {from} -> {to}"),
                });
            }
            for id in [from, to] {
                if !(imported_ids.contains(id.as_str()) || merge && sing.get(ns, id).is_some()) {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    async fn prepare_import_state(
        &self,
        ns: &str,
        concepts: &[Concept],
        associations: &[(String, String, f32)],
        merge: bool,
    ) -> Result<Singularity> {
        let (config, cache_metrics, existing_concepts, existing_associations) = {
            let sing = self.singularity.read().await;
            let existing_concepts = if merge {
                sing.all_concepts(ns)
            } else {
                Vec::new()
            };
            let existing_associations = if merge {
                sing.get_namespace(ns)
                    .map(|state| {
                        state
                            .associations
                            .iter()
                            .flat_map(|(from, neighbors)| {
                                neighbors.iter().map(|(to, (strength, created_at))| {
                                    (from.clone(), to.clone(), *strength, *created_at)
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            (
                sing.config.clone(),
                sing.cache_metrics.clone(),
                existing_concepts,
                existing_associations,
            )
        };

        let mut staged = Singularity::new_with_metrics(config, cache_metrics);
        for concept in existing_concepts
            .into_iter()
            .chain(concepts.iter().cloned())
        {
            staged.inject(ns, concept)?;
        }
        for (from, to, strength, created_at) in existing_associations {
            staged
                .ensure_namespace(ns)?
                .associations
                .entry(from)
                .or_default()
                .insert(to, (strength, created_at));
        }
        for (from, to, strength) in associations {
            staged.associate(ns, from, to, *strength)?;
        }
        Ok(staged)
    }
}
