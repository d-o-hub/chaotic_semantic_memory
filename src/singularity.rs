//! Episode-free concept injection

use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

const DEFAULT_CONCEPT_CACHE_SIZE: usize = 128;
pub const DEFAULT_MAX_CACHED_TOP_K: usize = 100;

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
/// Runtime configuration for [`Singularity`].
pub struct SingularityConfig {
    /// Maximum concept count before eviction (default: `None`).
    pub max_concepts: Option<usize>,
    /// Maximum outbound associations per concept (default: `None`).
    pub max_associations_per_concept: Option<usize>,
    /// LRU cache capacity for similarity results (default: `128`, coerced to `>= 1`).
    pub concept_cache_size: usize,
    /// Maximum top_k for cache eligibility (default: `100`).
    /// Queries with top_k > this value bypass the cache.
    pub max_cached_top_k: usize,
}

impl Default for SingularityConfig {
    fn default() -> Self {
        Self {
            max_concepts: None,
            max_associations_per_concept: None,
            concept_cache_size: DEFAULT_CONCEPT_CACHE_SIZE,
            max_cached_top_k: DEFAULT_MAX_CACHED_TOP_K,
        }
    }
}

#[derive(Debug, Default)]
struct QueryCache {
    capacity: usize,
    order: VecDeque<u64>,
    results: HashMap<u64, Arc<[(String, f32)]>>,
}

impl QueryCache {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            order: VecDeque::new(),
            results: HashMap::new(),
        }
    }

    fn get(&mut self, key: u64) -> Option<Arc<[(String, f32)]>> {
        let value = Arc::clone(self.results.get(&key)?);
        if let Some(pos) = self.order.iter().position(|k| *k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
        Some(value)
    }

    fn put(&mut self, key: u64, value: Arc<[(String, f32)]>) -> bool {
        if let Entry::Occupied(mut entry) = self.results.entry(key) {
            entry.insert(value);
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return false;
        }

        let mut evicted = false;
        if self.results.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.results.remove(&oldest);
                evicted = true;
            }
        }
        self.order.push_back(key);
        self.results.insert(key, value);
        evicted
    }

    fn clear(&mut self) {
        self.order.clear();
        self.results.clear();
    }
}

#[derive(Debug, Default)]
struct CacheMetrics {
    hits_total: AtomicU64,
    misses_total: AtomicU64,
    evictions_total: AtomicU64,
}

#[derive(Debug, Clone, Default)]
pub struct CacheMetricsSnapshot {
    pub cache_hits_total: u64,
    pub cache_misses_total: u64,
    pub cache_evictions_total: u64,
}

impl CacheMetrics {
    fn snapshot(&self) -> CacheMetricsSnapshot {
        CacheMetricsSnapshot {
            cache_hits_total: self.hits_total.load(Ordering::Relaxed),
            cache_misses_total: self.misses_total.load(Ordering::Relaxed),
            cache_evictions_total: self.evictions_total.load(Ordering::Relaxed),
        }
    }
}

/// Episode-free singularity engine
#[derive(Debug)]
pub struct Singularity {
    concepts: HashMap<String, Concept>,
    associations: HashMap<String, HashMap<String, f32>>,
    config: SingularityConfig,
    query_cache: RwLock<QueryCache>,
    cache_metrics: CacheMetrics,
}

impl Singularity {
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(SingularityConfig::default())
    }

    pub fn with_config(config: SingularityConfig) -> Self {
        Self {
            concepts: HashMap::new(),
            associations: HashMap::new(),
            query_cache: RwLock::new(QueryCache::with_capacity(config.concept_cache_size)),
            cache_metrics: CacheMetrics::default(),
            config,
        }
    }

    /// Inject a concept directly into memory
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, concept), fields(concept_id = %concept.id)))]
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
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
    pub fn get(&self, id: &str) -> Option<&Concept> {
        self.concepts.get(id)
    }

    /// Delete concept by ID
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
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
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, query), fields(top_k = top_k)))]
    pub fn find_similar(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        self.find_similar_cached(query, top_k).as_ref().to_vec()
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    ///
    /// Cache hits avoid cloning the cached result vector.
    /// Queries with `top_k > max_cached_top_k` bypass the cache to prevent
    /// excessive memory usage from storing large result sets.
    pub fn find_similar_cached(&self, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
        if top_k == 0 || self.concepts.is_empty() {
            return Arc::from(Vec::new());
        }

        // Bypass cache for large top_k to prevent excessive memory usage
        let bypass_cache = top_k > self.config.max_cached_top_k;

        if !bypass_cache {
            let cache_key = similarity_cache_key(query, top_k);
            if let Ok(mut cache) = self.query_cache.write() {
                if let Some(results) = cache.get(cache_key) {
                    self.cache_metrics
                        .hits_total
                        .fetch_add(1, Ordering::Relaxed);
                    return results;
                }
            }
            self.cache_metrics
                .misses_total
                .fetch_add(1, Ordering::Relaxed);
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
            if !bypass_cache {
                if let Ok(mut cache) = self.query_cache.write() {
                    let cache_key = similarity_cache_key(query, top_k);
                    let results = Arc::from(results);
                    if cache.put(cache_key, Arc::clone(&results)) {
                        self.cache_metrics
                            .evictions_total
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    return results;
                }
            }
            return Arc::from(results);
        }

        results.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
        results.truncate(top_k);
        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        if !bypass_cache {
            if let Ok(mut cache) = self.query_cache.write() {
                let cache_key = similarity_cache_key(query, top_k);
                let results = Arc::from(results);
                if cache.put(cache_key, Arc::clone(&results)) {
                    self.cache_metrics
                        .evictions_total
                        .fetch_add(1, Ordering::Relaxed);
                }
                return results;
            }
        }
        Arc::from(results)
    }

    /// Create or update association between concepts
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(from_id = %from, to_id = %to, strength = strength)))]
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
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(concept_id = %id)))]
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

    pub fn cache_metrics_snapshot(&self) -> CacheMetricsSnapshot {
        self.cache_metrics.snapshot()
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
        if let Ok(mut cache) = self.query_cache.write() {
            cache.clear();
        }
    }
}

impl Default for Singularity {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp in seconds
pub(crate) fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate cache key for similarity query
pub(crate) fn similarity_cache_key(query: &HVec10240, top_k: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    top_k.hash(&mut hasher);
    query.data.hash(&mut hasher);
    hasher.finish()
}

// Re-export ConceptBuilder from the dedicated module
pub use crate::concept_builder::ConceptBuilder;
