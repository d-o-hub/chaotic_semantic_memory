//! Extension methods for Singularity (extracted to satisfy LOC gate)

use tracing::instrument;
use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Singularity;

impl Singularity {
    /// Bundle multiple concepts into a single hypervector.
    #[instrument(skip(self, ns), fields(ids_count = ids.len()))]
    pub fn bundle_concepts_strict(&self, ns: &str, ids: &[String]) -> Result<HVec10240> {
        let ns_state = self.get_namespace(ns).ok_or_else(|| MemoryError::NotFound {
            entity: "Namespace".to_string(),
            id: ns.to_string(),
        })?;
        let mut vectors = Vec::with_capacity(ids.len());
        for id in ids {
            match ns_state.concepts.get(id) {
                Some(concept) => vectors.push(concept.vector),
                None => {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: id.clone(),
                    })
                }
            }
        }

        if vectors.is_empty() {
            return Err(MemoryError::InvalidInput {
                field: "ids".to_string(),
                reason: "Empty concept list for bundling".to_string(),
            });
        }

        HVec10240::bundle(&vectors)
    }

    pub fn update_metadata(
        &mut self,
        ns: &str,
        id: &str,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        if let Some(concept) = ns_state.concepts.get_mut(id) {
            concept.metadata = metadata;
            concept.modified_at = crate::singularity::unix_now_secs();
            self.invalidate_cache(ns);
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    pub fn clear_associations(&mut self, ns: &str, id: &str) -> Result<()> {
        let ns_state = self.get_namespace_mut(ns);
        if let Some(neighbors) = ns_state.associations.get_mut(id) {
            neighbors.clear();
        }
        Ok(())
    }
}
