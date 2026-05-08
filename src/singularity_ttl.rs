//! TTL (Time-To-Live) operations for Singularity.

use std::collections::HashMap;

use crate::hyperdim::Hypervector;
use crate::singularity::{Concept, Singularity, unix_now_secs};

impl<H: Hypervector> Default for Concept<H> {
    fn default() -> Self {
        Self {
            id: String::new(),
            vector: H::zero(),
            metadata: HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }
}

impl<H: Hypervector + 'static> Singularity<H> {
    /// Purge all expired concepts from memory.
    ///
    /// Returns the number of concepts removed.
    pub fn purge_expired(&mut self, ns: &str) -> usize {
        let now = unix_now_secs();
        let Some(ns_state) = self.get_namespace(ns) else {
            return 0;
        };
        let expired: Vec<String> = ns_state
            .concepts
            .iter()
            .filter(|(_, c)| c.expires_at.is_some_and(|exp| exp <= now))
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            self.delete(ns, &id).ok();
        }
        if count > 0 {
            self.invalidate_cache(ns);
        }
        count
    }

    /// Check if a concept is expired.
    pub fn is_expired(&self, ns: &str, id: &str) -> bool {
        let now = unix_now_secs();
        self.get(ns, id)
            .is_some_and(|c| c.expires_at.is_some_and(|exp| exp <= now))
    }

    /// Get all non-expired concept IDs.
    pub fn active_concept_ids(&self, ns: &str) -> Vec<String> {
        let now = unix_now_secs();
        self.get_namespace(ns)
            .map(|n| {
                n.concepts
                    .iter()
                    .filter(|(_, c)| c.expires_at.is_none_or(|exp| exp > now))
                    .map(|(id, _)| id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;
    use crate::singularity::SingularityConfig;

    #[test]
    fn test_purge_expired() {
        let mut sing: Singularity<HVec10240> = Singularity::new(SingularityConfig::default());
        let now = unix_now_secs();

        let concept1 = Concept {
            id: "expired".to_string(),
            expires_at: Some(now - 100),
            ..Default::default()
        };

        let concept2 = Concept {
            id: "active".to_string(),
            expires_at: Some(now + 100),
            ..Default::default()
        };

        let concept3 = Concept {
            id: "no_exp".to_string(),
            expires_at: None,
            ..Default::default()
        };

        let ns = "_default";
        sing.inject("_default", concept1).unwrap();
        sing.inject("_default", concept2).unwrap();
        sing.inject("_default", concept3).unwrap();

        assert_eq!(sing.active_concept_ids(ns).len(), 2);

        let purged = sing.purge_expired("_default");
        assert_eq!(purged, 1);

        assert!(
            !sing
                .get_namespace(ns)
                .unwrap()
                .concepts
                .contains_key("expired")
        );
        assert!(
            sing.get_namespace(ns)
                .unwrap()
                .concepts
                .contains_key("active")
        );
        assert!(
            sing.get_namespace(ns)
                .unwrap()
                .concepts
                .contains_key("no_exp")
        );
    }

    #[test]
    fn test_is_expired() {
        let mut sing: Singularity<HVec10240> = Singularity::new(SingularityConfig::default());
        let now = unix_now_secs();

        let concept1 = Concept {
            id: "expired".to_string(),
            expires_at: Some(now - 100),
            ..Default::default()
        };

        let concept2 = Concept {
            id: "active".to_string(),
            expires_at: Some(now + 100),
            ..Default::default()
        };

        let concept3 = Concept {
            id: "no_exp".to_string(),
            expires_at: None,
            ..Default::default()
        };

        let concept4 = Concept {
            id: "just_expired".to_string(),
            expires_at: Some(now),
            ..Default::default()
        };

        let ns = "_default";
        sing.inject("_default", concept1).unwrap();
        sing.inject("_default", concept2).unwrap();
        sing.inject("_default", concept3).unwrap();
        sing.inject("_default", concept4).unwrap();

        assert!(sing.is_expired(ns, "expired"));
        assert!(!sing.is_expired(ns, "active"));
        assert!(!sing.is_expired(ns, "no_exp"));
        assert!(sing.is_expired(ns, "just_expired"));
        assert!(!sing.is_expired(ns, "nonexistent"));
    }

    #[test]
    fn test_active_concept_ids() {
        let mut sing: Singularity<HVec10240> = Singularity::new(SingularityConfig::default());
        let now = unix_now_secs();

        let concept1 = Concept {
            id: "expired".to_string(),
            expires_at: Some(now - 100),
            ..Default::default()
        };

        let concept2 = Concept {
            id: "active".to_string(),
            expires_at: Some(now + 100),
            ..Default::default()
        };

        let concept3 = Concept {
            id: "no_exp".to_string(),
            expires_at: None,
            ..Default::default()
        };

        let ns = "_default";
        sing.inject("_default", concept1).unwrap();
        sing.inject("_default", concept2).unwrap();
        sing.inject("_default", concept3).unwrap();

        let mut active = sing.active_concept_ids(ns);
        active.sort();

        let mut expected = vec!["active".to_string(), "no_exp".to_string()];
        expected.sort();

        assert_eq!(active, expected);
    }
}
