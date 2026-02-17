//! Episode-free concept injection

use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

/// A concept in semantic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
}

#[derive(Debug, Clone)]
pub struct SingularityConfig {
    pub max_concepts: Option<usize>,
    pub max_associations_per_concept: Option<usize>,
    pub concept_cache_size: usize,
}

impl Default for SingularityConfig {
    fn default() -> Self {
        Self {
            max_concepts: None,
            max_associations_per_concept: None,
            concept_cache_size: 1000,
        }
    }
}

#[derive(Default)]
struct QueryCache {
    capacity: usize,
    order: VecDeque<u64>,
    results: HashMap<u64, Vec<(String, f32)>>,
}

impl QueryCache {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            results: HashMap::new(),
        }
    }

    fn get(&mut self, key: u64) -> Option<Vec<(String, f32)>> {
        let value = self.results.get(&key).cloned()?;
        if let Some(pos) = self.order.iter().position(|k| *k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
        Some(value)
    }

    fn put(&mut self, key: u64, value: Vec<(String, f32)>) {
        if let Entry::Occupied(mut entry) = self.results.entry(key) {
            entry.insert(value);
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return;
        }

        if self.results.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.results.remove(&oldest);
            }
        }
        self.order.push_back(key);
        self.results.insert(key, value);
    }

    fn clear(&mut self) {
        self.order.clear();
        self.results.clear();
    }
}

/// Episode-free singularity engine
pub struct Singularity {
    concepts: HashMap<String, Concept>,
    associations: HashMap<String, HashMap<String, f32>>,
    config: SingularityConfig,
    query_cache: Mutex<QueryCache>,
}

impl Singularity {
    pub fn new() -> Self {
        Self::with_config(SingularityConfig::default())
    }

    pub fn with_config(config: SingularityConfig) -> Self {
        Self {
            concepts: HashMap::new(),
            associations: HashMap::new(),
            query_cache: Mutex::new(QueryCache::with_capacity(config.concept_cache_size)),
            config,
        }
    }

    /// Inject a concept directly into memory
    pub fn inject(&mut self, concept: Concept) -> Result<()> {
        if concept.vector.data.len() != 80 {
            return Err(MemoryError::InvalidDimension {
                expected: 80,
                actual: concept.vector.data.len(),
            });
        }

        let is_new = !self.concepts.contains_key(&concept.id);
        if is_new {
            self.evict_oldest_if_needed();
        }

        self.concepts.insert(concept.id.clone(), concept);
        self.invalidate_cache();
        Ok(())
    }

    /// Retrieve concept by ID
    pub fn get(&self, id: &str) -> Option<&Concept> {
        self.concepts.get(id)
    }

    /// Delete concept by ID
    pub fn delete(&mut self, id: &str) -> Result<()> {
        self.concepts.remove(id);
        self.associations.remove(id);
        for links in self.associations.values_mut() {
            links.remove(id);
        }
        self.invalidate_cache();
        Ok(())
    }

    /// Clear all concepts and associations
    pub fn clear(&mut self) {
        self.concepts.clear();
        self.associations.clear();
        self.invalidate_cache();
    }

    /// Update concept vector
    pub fn update(&mut self, id: &str, new_vector: HVec10240) -> Result<()> {
        if let Some(concept) = self.concepts.get_mut(id) {
            concept.vector = new_vector;
            concept.modified_at = unix_now_secs();
            self.invalidate_cache();
            Ok(())
        } else {
            Err(MemoryError::Persistence(format!(
                "Concept '{}' not found",
                id
            )))
        }
    }

    /// Find similar concepts using cosine similarity
    pub fn find_similar(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        if top_k == 0 || self.concepts.is_empty() {
            return Vec::new();
        }

        let cache_key = similarity_cache_key(query, top_k);
        if let Ok(mut cache) = self.query_cache.lock() {
            if let Some(results) = cache.get(cache_key) {
                return results;
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let mut results: Vec<(String, f32)> = self
            .concepts
            .values()
            .par_bridge()
            .map(|c| (c.id.clone(), query.cosine_similarity(&c.vector)))
            .collect();

        #[cfg(target_arch = "wasm32")]
        let mut results: Vec<(String, f32)> = self
            .concepts
            .values()
            .map(|c| (c.id.clone(), query.cosine_similarity(&c.vector)))
            .collect();

        if results.len() <= top_k {
            results.sort_by(|a, b| b.1.total_cmp(&a.1));
            if let Ok(mut cache) = self.query_cache.lock() {
                cache.put(cache_key, results.clone());
            }
            return results;
        }

        results.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
        results.truncate(top_k);
        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        if let Ok(mut cache) = self.query_cache.lock() {
            cache.put(cache_key, results.clone());
        }
        results
    }

    /// Create or update association between concepts
    pub fn associate(&mut self, from: &str, to: &str, strength: f32) -> Result<()> {
        if !self.concepts.contains_key(from) || !self.concepts.contains_key(to) {
            return Err(MemoryError::Persistence(
                "Both concepts must exist to create association".to_string(),
            ));
        }

        let links = self.associations.entry(from.to_string()).or_default();
        links.insert(to.to_string(), strength);

        if let Some(limit) = self.config.max_associations_per_concept {
            while links.len() > limit {
                if let Some((weakest, _)) = links
                    .iter()
                    .min_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(k, v)| (k.clone(), *v))
                {
                    links.remove(&weakest);
                } else {
                    break;
                }
            }
        }

        self.invalidate_cache();
        Ok(())
    }

    /// Get associations for a concept
    pub fn get_associations(&self, id: &str) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = self
            .associations
            .get(id)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default();
        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        results
    }

    /// Bundle multiple concepts into a single hypervector
    pub fn bundle_concepts(&self, ids: &[String]) -> Result<HVec10240> {
        let vectors: Vec<_> = ids
            .iter()
            .filter_map(|id| self.concepts.get(id))
            .map(|c| c.vector)
            .collect();

        HVec10240::bundle(&vectors)
    }

    pub fn concept_ids(&self) -> Vec<String> {
        self.concepts.keys().cloned().collect()
    }

    pub fn all_concepts(&self) -> Vec<Concept> {
        self.concepts.values().cloned().collect()
    }

    pub fn all_associations(&self) -> Vec<(String, String, f32)> {
        let mut output = Vec::new();
        for (from, links) in &self.associations {
            for (to, strength) in links {
                output.push((from.clone(), to.clone(), *strength));
            }
        }
        output
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    fn evict_oldest_if_needed(&mut self) {
        let Some(limit) = self.config.max_concepts else {
            return;
        };

        while self.concepts.len() >= limit {
            let oldest = self
                .concepts
                .values()
                .min_by_key(|c| c.created_at)
                .map(|c| c.id.clone());

            if let Some(oldest_id) = oldest {
                self.concepts.remove(&oldest_id);
                self.associations.remove(&oldest_id);
                for links in self.associations.values_mut() {
                    links.remove(&oldest_id);
                }
                self.invalidate_cache();
            } else {
                break;
            }
        }
    }

    fn invalidate_cache(&self) {
        if let Ok(mut cache) = self.query_cache.lock() {
            cache.clear();
        }
    }
}

impl Default for Singularity {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating concepts
pub struct ConceptBuilder {
    id: String,
    vector: Option<HVec10240>,
    metadata: HashMap<String, serde_json::Value>,
    metadata_error: Option<MemoryError>,
}

impl ConceptBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            vector: None,
            metadata: HashMap::new(),
            metadata_error: None,
        }
    }

    pub fn with_vector(mut self, vector: HVec10240) -> Self {
        self.vector = Some(vector);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if self.metadata_error.is_none() {
            match serde_json::to_value(value) {
                Ok(value) => {
                    self.metadata.insert(key.into(), value);
                }
                Err(error) => {
                    self.metadata_error = Some(MemoryError::Serialization(error));
                }
            }
        }
        self
    }

    pub fn build(self) -> Result<Concept> {
        if let Some(error) = self.metadata_error {
            return Err(error);
        }

        let now = unix_now_secs();

        Ok(Concept {
            id: self.id,
            vector: self.vector.unwrap_or_else(HVec10240::random),
            metadata: self.metadata,
            created_at: now,
            modified_at: now,
        })
    }
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn similarity_cache_key(query: &HVec10240, top_k: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    top_k.hash(&mut hasher);
    query.to_bytes().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use serde::ser::{Error as _, Serializer};
    use serde::Serialize;

    use super::*;

    struct FailingMetadata;

    impl Serialize for FailingMetadata {
        fn serialize<S>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(S::Error::custom(
                "intentional metadata serialization failure",
            ))
        }
    }

    #[test]
    fn concept_builder_returns_error_when_metadata_serialization_fails() {
        let result = ConceptBuilder::new("failing")
            .with_metadata("bad", FailingMetadata)
            .build();
        assert!(result.is_err());
    }
}
