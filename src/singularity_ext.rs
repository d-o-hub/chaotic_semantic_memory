//! Singularity extension methods for API completeness.

use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::{Singularity, unix_now_secs};

impl Singularity {
    /// Bundle multiple concepts into a single hypervector (strict version).
    /// Returns `NotFound` error if any concept ID is missing.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(ids_count = ids.len()))
    )]
    pub fn bundle_concepts_strict(&self, ids: &[String]) -> Result<HVec10240> {
        let mut vectors = Vec::with_capacity(ids.len());
        for id in ids {
            match self.concepts.get(id) {
                Some(concept) => vectors.push(concept.vector),
                None => {
                    return Err(MemoryError::NotFound {
                        entity: "Concept".to_string(),
                        id: id.clone(),
                    });
                }
            }
        }
        HVec10240::bundle(&vectors)
    }

    /// Remove an association between two concepts.
    /// Returns Ok(()) even if the association didn't exist.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(from_id = %from, to_id = %to))
    )]
    pub fn disassociate(&mut self, from: &str, to: &str) -> Result<()> {
        if !self.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if let Some(links) = self.associations.get_mut(from) {
            links.remove(to);
        }
        self.invalidate_cache();
        Ok(())
    }

    /// Clear all outbound associations for a concept.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(concept_id = %id))
    )]
    pub fn clear_associations(&mut self, id: &str) -> Result<()> {
        if !self.concepts.contains_key(id) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            });
        }
        self.associations.remove(id);
        self.invalidate_cache();
        Ok(())
    }

    /// Clear the similarity query cache.
    pub fn clear_similarity_cache(&self) {
        self.invalidate_cache();
    }

    /// Update concept metadata.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        instrument(skip(self), fields(concept_id = %id))
    )]
    pub fn update_metadata(
        &mut self,
        id: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        if let Some(concept) = self.concepts.get_mut(id) {
            concept.metadata = metadata;
            concept.modified_at = unix_now_secs();
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }
}
