//! Episode-free concept injection

use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::error::{MemoryError, Result};
use crate::hyperdim::HVec10240;

// Re-export retrieval types from the dedicated module
use crate::singularity_retrieval::ScoredCandidateParams;
pub use crate::singularity_retrieval::{CandidateSource, RetrievalConfig, RetrievalStats};

const DEFAULT_CONCEPT_CACHE_SIZE: usize = 128;
pub const DEFAULT_MAX_CACHED_TOP_K: usize = 100;

/// A concept in semantic memory.
///
/// Use [`ConceptBuilder`] to construct instances with proper defaults.
/// Direct struct construction is supported but the `expires_at` field should
/// default to `None` for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    #[serde(default)]
    pub expires_at: Option<u64>,
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
pub(crate) struct QueryCache {
    pub(crate) capacity: usize,
    pub(crate) order: VecDeque<u64>,
    pub(crate) results: HashMap<u64, Arc<[(String, f32)]>>,
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

    pub(crate) fn put(&mut self, key: u64, value: Arc<[(String, f32)]>) -> bool {
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
pub(crate) struct CacheMetrics {
    pub(crate) hits_total: AtomicU64,
    pub(crate) misses_total: AtomicU64,
    pub(crate) evictions_total: AtomicU64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub(crate) concepts: HashMap<String, Concept>,
    pub(crate) associations: HashMap<String, HashMap<String, f32>>,
    pub(crate) concept_indices: Vec<String>,
    pub(crate) concept_vectors: Vec<HVec10240>,
    pub(crate) id_to_index: HashMap<String, usize>,
    pub(crate) config: SingularityConfig,
    pub(crate) retrieval_config: RetrievalConfig,
    pub(crate) query_cache: RwLock<QueryCache>,
    pub(crate) cache_metrics: CacheMetrics,
    pub(crate) last_retrieval_stats: RwLock<RetrievalStats>,
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
            concept_indices: Vec::new(),
            concept_vectors: Vec::new(),
            id_to_index: HashMap::new(),
            query_cache: RwLock::new(QueryCache::with_capacity(config.concept_cache_size)),
            cache_metrics: CacheMetrics::default(),
            last_retrieval_stats: RwLock::new(RetrievalStats::default()),
            config,
            retrieval_config: RetrievalConfig::default(),
        }
    }

    /// Inject a concept directly into memory
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, concept), fields(concept_id = %concept.id)))]
    pub fn inject(&mut self, concept: Concept) -> Result<()> {
        let is_new = !self.concepts.contains_key(&concept.id);
        if is_new {
            self.evict_oldest_if_needed();
        }

        if let Some(&idx) = self.id_to_index.get(&concept.id) {
            self.concept_vectors[idx] = concept.vector;
        } else {
            let idx = self.concept_indices.len();
            self.id_to_index.insert(concept.id.clone(), idx);
            self.concept_indices.push(concept.id.clone());
            self.concept_vectors.push(concept.vector);
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
        if let Some(idx) = self.id_to_index.remove(id) {
            self.concept_indices.swap_remove(idx);
            let _ = self.concept_vectors.swap_remove(idx);
            if idx < self.concept_indices.len() {
                let swapped_id = &self.concept_indices[idx];
                self.id_to_index.insert(swapped_id.clone(), idx);
            }
        }

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
        self.concept_indices.clear();
        self.concept_vectors.clear();
        self.id_to_index.clear();
        self.invalidate_cache();
    }

    /// Update concept vector
    pub fn update(&mut self, id: &str, new_vector: HVec10240) -> Result<()> {
        if let Some(&idx) = self.id_to_index.get(id) {
            self.concept_vectors[idx] = new_vector;
        }

        if let Some(concept) = self.concepts.get_mut(id) {
            concept.vector = new_vector;
            concept.modified_at = unix_now_secs();
            self.invalidate_cache();
            Ok(())
        } else {
            Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: id.to_string(),
            })
        }
    }

    /// Find similar concepts using cosine similarity
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, query), fields(top_k = top_k)))]
    pub fn find_similar(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        self.find_similar_arc(query, top_k).as_ref().to_vec()
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_arc(&self, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
        self.find_similar_cached(query, top_k)
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_cached(&self, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
        let start_ns = unix_now_ns();
        if top_k == 0 || self.concepts.is_empty() {
            let stats = RetrievalStats {
                fell_back_to_exact_scan: true,
                ..Default::default()
            };
            if let Ok(mut s) = self.last_retrieval_stats.write() {
                *s = stats;
            }
            return Arc::from(Vec::new());
        }

        let bypass_cache = top_k > self.config.max_cached_top_k;

        if !bypass_cache {
            let cache_key = similarity_cache_key(query, top_k);
            if let Ok(mut cache) = self.query_cache.write() {
                if let Some(results) = cache.get(cache_key) {
                    self.cache_metrics
                        .hits_total
                        .fetch_add(1, Ordering::Relaxed);
                    let stats = RetrievalStats {
                        candidate_count: results.len(),
                        scored_count: 0,
                        scoring_ns: unix_now_ns().saturating_sub(start_ns),
                        ..Default::default()
                    };
                    if let Ok(mut s) = self.last_retrieval_stats.write() {
                        *s = stats;
                    }
                    return results;
                }
            }
            self.cache_metrics
                .misses_total
                .fetch_add(1, Ordering::Relaxed);
        }

        // Generate candidates based on RetrievalConfig
        let candidate_start = unix_now_ns();
        let mut candidates = Vec::new();
        let mut source = CandidateSource::ExactFallback;

        if self.retrieval_config.enable_graph_candidates {
            candidates = self.generate_graph_candidates(query);
            if !candidates.is_empty() {
                source = CandidateSource::Graph;
            }
        }

        if candidates.is_empty() && self.retrieval_config.enable_bucket_candidates {
            candidates = self.generate_bucket_candidates(query);
            if !candidates.is_empty() {
                source = CandidateSource::Bucket;
            }
        }

        let cand_ns = unix_now_ns().saturating_sub(candidate_start);

        if candidates.is_empty() {
            return self.exact_similarity_scan(query, top_k, start_ns, bypass_cache);
        }

        // Reduced-candidate path
        self.scored_candidate_retrieval(ScoredCandidateParams {
            query,
            top_k,
            candidates,
            start_ns,
            cand_ns,
            source,
            bypass_cache,
        })
    }

    /// Create or update association between concepts
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self), fields(from_id = %from, to_id = %to, strength = strength)))]
    pub fn associate(&mut self, from: &str, to: &str, strength: f32) -> Result<()> {
        if !self.concepts.contains_key(from) || !self.concepts.contains_key(to) {
            let missing = if !self.concepts.contains_key(from) {
                from
            } else {
                to
            };
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: missing.to_string(),
            });
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
                self.delete(&oldest_id).ok();
                self.invalidate_cache();
            } else {
                break;
            }
        }
    }

    pub(crate) fn invalidate_cache(&self) {
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

/// Get current Unix timestamp in nanoseconds
pub(crate) fn unix_now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
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
