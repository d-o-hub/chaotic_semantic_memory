//! Time-To-Live (TTL) and concept expiration for Singularity.
//!
//! Extracted from singularity.rs to satisfy the 500 LOC gate.


use crate::hyperdim::Hypervector;
use crate::singularity::Singularity;
use crate::singularity::unix_now_secs;
impl<H: Hypervector + 'static> Singularity<H> {
    /// Remove expired concepts from the given namespace.
    /// Returns the number of concepts removed.
    pub fn purge_expired(&mut self, ns: &str) -> usize {
        let now = unix_now_secs();
        let expired_ids: Vec<String> = if let Some(ns_state) = self.get_namespace(ns) {
            ns_state
                .concepts
                .values()
                .filter(|c| c.expires_at.is_some_and(|t| t <= now))
                .map(|c| c.id.clone())
                .collect()
        } else {
            return 0;
        };

        let count = expired_ids.len();
        for id in expired_ids {
            let _ = self.delete(ns, &id);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singularity::{Singularity, SingularityConfig};
    use std::collections::HashMap;

    #[test]
    fn purge_expired_concepts() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        let now = unix_now_secs();

        // Expired
        let concept1 = Concept {
            id: "c1".to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: now,
            modified_at: now,
            expires_at: Some(now - 10),
            canonical_concept_ids: Vec::new(),
        };

        // Not expired
        let concept2 = Concept {
            id: "c2".to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: now,
            modified_at: now,
            expires_at: Some(now + 10),
            canonical_concept_ids: Vec::new(),
        };

        // No TTL
        let concept3 = Concept {
            id: "c3".to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: now,
            modified_at: now,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };

        sing.inject("_default", concept1).unwrap();
        sing.inject("_default", concept2).unwrap();
        sing.inject("_default", concept3).unwrap();

        assert_eq!(sing.len("_default"), 3);
        let purged = sing.purge_expired("_default");
        assert_eq!(purged, 1);
        assert_eq!(sing.len("_default"), 2);
        assert!(sing.get("_default", "c1").is_none());
    }

    #[test]
    fn purge_expired_empty_or_missing_ns() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        assert_eq!(sing.purge_expired("_default"), 0);
        assert_eq!(sing.purge_expired("missing"), 0);
    }
}
