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
