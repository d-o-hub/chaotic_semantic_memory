//! TTL (Time-To-Live) operations for Singularity.

use std::collections::HashMap;

use crate::hyperdim::HVec10240;
use crate::singularity::{Concept, Singularity, unix_now_secs};

impl Default for Concept {
    fn default() -> Self {
        Self {
            id: String::new(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 0,
            modified_at: 0,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        }
    }
}

impl Singularity {
    /// Purge all expired concepts from memory.
    ///
    /// Returns the number of concepts removed.
    pub fn purge_expired(&mut self) -> usize {
        let now = unix_now_secs();
        let expired: Vec<String> = self
            .concepts
            .iter()
            .filter(|(_, c)| c.expires_at.is_some_and(|exp| exp <= now))
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in expired {
            self.delete(&id).ok();
        }
        if count > 0 {
            self.invalidate_cache();
        }
        count
    }

    /// Check if a concept is expired.
    pub fn is_expired(&self, id: &str) -> bool {
        let now = unix_now_secs();
        self.concepts
            .get(id)
            .is_some_and(|c| c.expires_at.is_some_and(|exp| exp <= now))
    }

    /// Get all non-expired concept IDs.
    pub fn active_concept_ids(&self) -> Vec<String> {
        let now = unix_now_secs();
        self.concepts
            .iter()
            .filter(|(_, c)| c.expires_at.is_none_or(|exp| exp > now))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purge_expired() {
        let mut sing = Singularity::new();
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

        sing.inject(concept1).unwrap();
        sing.inject(concept2).unwrap();
        sing.inject(concept3).unwrap();

        assert_eq!(sing.active_concept_ids().len(), 2);

        let purged = sing.purge_expired();
        assert_eq!(purged, 1);

        assert!(!sing.concepts.contains_key("expired"));
        assert!(sing.concepts.contains_key("active"));
        assert!(sing.concepts.contains_key("no_exp"));
    }

    #[test]
    fn test_is_expired() {
        let mut sing = Singularity::new();
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

        sing.inject(concept1).unwrap();
        sing.inject(concept2).unwrap();
        sing.inject(concept3).unwrap();
        sing.inject(concept4).unwrap();

        assert!(sing.is_expired("expired"));
        assert!(!sing.is_expired("active"));
        assert!(!sing.is_expired("no_exp"));
        assert!(sing.is_expired("just_expired"));
        assert!(!sing.is_expired("nonexistent"));
    }

    #[test]
    fn test_active_concept_ids() {
        let mut sing = Singularity::new();
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

        sing.inject(concept1).unwrap();
        sing.inject(concept2).unwrap();
        sing.inject(concept3).unwrap();

        let mut active = sing.active_concept_ids();
        active.sort();

        let mut expected = vec!["active".to_string(), "no_exp".to_string()];
        expected.sort();

        assert_eq!(active, expected);
    }
}
