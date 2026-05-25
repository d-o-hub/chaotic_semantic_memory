//! Extension methods for Singularity (extracted to satisfy LOC gate)

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;
use crate::singularity::Singularity;
use tracing::instrument;

impl Singularity {
    /// Bundle multiple concepts into a single hypervector.
    #[instrument(skip(self, ns), fields(ids_count = ids.len()))]
    pub fn bundle_concepts_strict(&self, ns: &str, ids: &[String]) -> Result<HVec10240> {
        let ns_state = self
            .get_namespace(ns)
            .ok_or_else(|| MemoryError::NotFound {
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
                    });
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

    /// Process all concepts in a namespace using a callback.
    pub fn for_each_concept<F>(&self, ns: &str, mut f: F)
    where
        F: FnMut(&crate::singularity::Concept),
    {
        if let Some(ns_state) = self.get_namespace(ns) {
            for concept in ns_state.concepts.values() {
                f(concept);
            }
        }
    }

    /// Process all associations in a namespace using a callback.
    pub fn for_each_association<F>(&self, ns: &str, mut f: F)
    where
        F: FnMut(&String, &String, f32),
    {
        if let Some(ns_state) = self.get_namespace(ns) {
            for (from_id, neighbors) in &ns_state.associations {
                for (to_id, strength) in neighbors {
                    f(from_id, to_id, *strength);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MemoryError;
    use crate::singularity::{ConceptBuilder, Singularity, SingularityConfig};
    use std::collections::HashMap;

    const NS: &str = "_default";

    #[test]
    fn test_bundle_concepts_strict_success() -> crate::error::Result<()> {
        let mut singularity = Singularity::with_config(SingularityConfig::default());
        let vec1 = HVec10240::random();
        let vec2 = HVec10240::random();

        let c1 = ConceptBuilder::new("c1").with_vector(vec1).build().unwrap();
        let c2 = ConceptBuilder::new("c2").with_vector(vec2).build().unwrap();

        singularity.inject(NS, c1)?;
        singularity.inject(NS, c2)?;

        let result = singularity.bundle_concepts_strict(NS, &["c1".to_string(), "c2".to_string()]);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_bundle_concepts_strict_missing_id() -> crate::error::Result<()> {
        let mut singularity = Singularity::with_config(SingularityConfig::default());
        let vec1 = HVec10240::random();

        let c1 = ConceptBuilder::new("c1").with_vector(vec1).build()?;
        singularity.inject(NS, c1)?;

        let result =
            singularity.bundle_concepts_strict(NS, &["c1".to_string(), "missing_id".to_string()]);

        match result {
            Err(MemoryError::NotFound { entity, id }) => {
                assert_eq!(entity, "Concept");
                assert_eq!(id, "missing_id");
            }
            _ => panic!("Expected NotFound error, got {result:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_update_metadata_not_found() {
        let mut sing = Singularity::new(SingularityConfig::default());
        let metadata = HashMap::new();

        let result = sing.update_metadata(NS, "non-existent-id", metadata);

        match result {
            Err(MemoryError::NotFound { entity, id }) => {
                assert_eq!(entity, "Concept");
                assert_eq!(id, "non-existent-id");
            }
            _ => panic!("Expected MemoryError::NotFound, got {result:?}"),
        }
    }

    #[test]
    fn test_update_metadata_success() -> crate::error::Result<()> {
        let mut sing = Singularity::new(SingularityConfig::default());
        let concept = ConceptBuilder::new("test-id")
            .with_metadata("original", serde_json::Value::Bool(true))
            .build()
            .expect("Failed to build concept");

        sing.inject(NS, concept)?;

        let mut new_metadata = HashMap::new();
        new_metadata.insert("updated".to_string(), serde_json::Value::Bool(true));

        let time_before = crate::singularity::unix_now_secs();

        let result = sing.update_metadata(NS, "test-id", new_metadata.clone());
        assert!(result.is_ok());

        let updated_concept = sing
            .get(NS, "test-id")
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: "test-id".to_string(),
            })?;
        assert_eq!(updated_concept.metadata, new_metadata);
        assert!(updated_concept.modified_at >= time_before);
        Ok(())
    }
}
