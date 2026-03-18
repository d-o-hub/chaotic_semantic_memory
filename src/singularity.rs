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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheMetricsSnapshot {
    pub cache_hits_total: u64,
    pub cache_misses_total: u64,
    pub cache_evictions_total: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievalStats {
    pub candidate_count: usize,
    pub scored_count: usize,
    pub fell_back_to_exact_scan: bool,
    pub candidate_ns: u64,
    pub scoring_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    Metadata,
    Graph,
    Bucket,
    ExactFallback,
}

pub(crate) struct ScoredCandidateParams<'a> {
    pub query: &'a HVec10240,
    pub top_k: usize,
    pub candidates: Vec<usize>,
    pub start_ns: u64,
    pub cand_ns: u64,
    pub source: CandidateSource,
    pub bypass_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub max_candidates: usize,
    pub candidate_ratio_fallback: f32,
    pub graph_depth: u8,
    pub graph_fanout: usize,
    pub bucket_probe_width: usize,
    pub enable_graph_candidates: bool,
    pub enable_bucket_candidates: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_candidates: 1000,
            candidate_ratio_fallback: 0.5,
            graph_depth: 2,
            graph_fanout: 10,
            bucket_probe_width: 2,
            enable_graph_candidates: false,
            enable_bucket_candidates: false,
        }
    }
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
    config: SingularityConfig,
    retrieval_config: RetrievalConfig,
    query_cache: RwLock<QueryCache>,
    cache_metrics: CacheMetrics,
    last_retrieval_stats: RwLock<RetrievalStats>,
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

    pub fn set_retrieval_config(&mut self, config: RetrievalConfig) {
        self.retrieval_config = config;
    }

    pub fn retrieval_config(&self) -> &RetrievalConfig {
        &self.retrieval_config
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
    ///
    /// Returns the cached result directly without cloning, avoiding unnecessary
    /// Vec allocation on cache hits.
    ///
    /// Cache hits avoid cloning the cached result vector.
    /// Queries with `top_k > max_cached_top_k` bypass the cache to prevent
    /// excessive memory usage from storing large result sets.
    pub fn find_similar_arc(&self, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
        self.find_similar_cached(query, top_k)
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    ///
    /// Cache hits avoid cloning the cached result vector.
    /// Queries with `top_k > max_cached_top_k` bypass the cache to prevent
    /// excessive memory usage from storing large result sets.
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

        // Bypass cache for large top_k to prevent excessive memory usage
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

    pub(crate) fn scored_candidate_retrieval(
        &self,
        params: ScoredCandidateParams,
    ) -> Arc<[(String, f32)]> {
        let ScoredCandidateParams {
            query,
            top_k,
            candidates,
            start_ns: _start_ns,
            cand_ns,
            source: _source,
            bypass_cache,
        } = params;
        let scoring_start = unix_now_ns();
        let candidate_count = candidates.len();

        #[cfg(not(target_arch = "wasm32"))]
        let mut scores: Vec<(usize, f32)> = candidates
            .into_par_iter()
            .map(|idx| (idx, query.cosine_similarity(&self.concept_vectors[idx])))
            .collect();

        #[cfg(target_arch = "wasm32")]
        let mut scores: Vec<(usize, f32)> = candidates
            .into_iter()
            .map(|idx| (idx, query.cosine_similarity(&self.concept_vectors[idx])))
            .collect();

        let scoring_ns = unix_now_ns().saturating_sub(scoring_start);
        let scored_count = scores.len();

        if scores.len() <= top_k {
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
            scores.truncate(top_k);
            scores.sort_by(|a, b| b.1.total_cmp(&a.1));
        }

        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(idx, score)| (self.concept_indices[idx].clone(), score))
            .collect();

        let results_arc = Arc::from(results);
        if !bypass_cache {
            if let Ok(mut cache) = self.query_cache.write() {
                let cache_key = similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    self.cache_metrics
                        .evictions_total
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        self.update_stats(candidate_count, scored_count, false, cand_ns, scoring_ns);

        results_arc
    }

    fn generate_graph_candidates(&self, query: &HVec10240) -> Vec<usize> {
        let mut candidates = std::collections::HashSet::new();
        // Use top-1 as seeds for graph expansion
        let results = self.exact_similarity_scan(query, 1, unix_now_ns(), true);
        if let Some((seed_id, _)) = results.first() {
            let mut queue = VecDeque::new();
            queue.push_back((seed_id.clone(), 0u8));
            candidates.insert(seed_id.clone());

            while let Some((id, depth)) = queue.pop_front() {
                if depth >= self.retrieval_config.graph_depth {
                    continue;
                }
                if let Some(links) = self.associations.get(&id) {
                    let mut sorted_links: Vec<_> = links.iter().collect();
                    sorted_links.sort_by(|a, b| b.1.total_cmp(a.1));
                    for (neighbor_id, _) in sorted_links
                        .into_iter()
                        .take(self.retrieval_config.graph_fanout)
                    {
                        if !candidates.contains(neighbor_id) {
                            candidates.insert(neighbor_id.clone());
                            queue.push_back((neighbor_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        candidates
            .into_iter()
            .filter_map(|id| self.id_to_index.get(&id).copied())
            .collect()
    }

    fn generate_bucket_candidates(&self, query: &HVec10240) -> Vec<usize> {
        // Coarse bucketing: use first N bits of the hypervector as bucket ID
        let bucket_count = 1 << self.retrieval_config.bucket_probe_width;
        let mut bucket_candidates = Vec::new();

        // Very simple bucketing: XOR first word and take modulo
        let query_bucket = (query.data[0] % bucket_count as u128) as usize;

        for (idx, vec) in self.concept_vectors.iter().enumerate() {
            let vec_bucket = (vec.data[0] % bucket_count as u128) as usize;
            if vec_bucket == query_bucket {
                bucket_candidates.push(idx);
            }
        }

        bucket_candidates
    }

    fn exact_similarity_scan(
        &self,
        query: &HVec10240,
        top_k: usize,
        start_ns: u64,
        bypass_cache: bool,
    ) -> Arc<[(String, f32)]> {
        let scoring_start = unix_now_ns();
        #[cfg(not(target_arch = "wasm32"))]
        let scores: Vec<f32> = self
            .concept_vectors
            .par_iter()
            .map(|v| query.cosine_similarity(v))
            .collect();

        #[cfg(target_arch = "wasm32")]
        let scores: Vec<f32> = self
            .concept_vectors
            .iter()
            .map(|v| query.cosine_similarity(v))
            .collect();

        let scoring_ns = unix_now_ns().saturating_sub(scoring_start);
        let scored_count = scores.len();

        let mut indices: Vec<usize> = (0..scored_count).collect();

        if scored_count <= top_k {
            indices.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        } else {
            indices.select_nth_unstable_by(top_k - 1, |&a, &b| scores[b].total_cmp(&scores[a]));
            indices.truncate(top_k);
            indices.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        }

        let results: Vec<(String, f32)> = indices
            .into_iter()
            .map(|idx| (self.concept_indices[idx].clone(), scores[idx]))
            .collect();

        let results_arc = Arc::from(results);
        if !bypass_cache {
            if let Ok(mut cache) = self.query_cache.write() {
                let cache_key = similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    self.cache_metrics
                        .evictions_total
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        self.update_stats(
            scored_count,
            scored_count,
            true,
            scoring_start.saturating_sub(start_ns),
            scoring_ns,
        );
        results_arc
    }

    fn update_stats(
        &self,
        candidates: usize,
        scored: usize,
        fallback: bool,
        cand_ns: u64,
        score_ns: u64,
    ) {
        let stats = RetrievalStats {
            candidate_count: candidates,
            scored_count: scored,
            fell_back_to_exact_scan: fallback,
            candidate_ns: cand_ns,
            scoring_ns: score_ns,
        };
        if let Ok(mut s) = self.last_retrieval_stats.write() {
            *s = stats;
        }
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

    pub fn last_retrieval_stats(&self) -> RetrievalStats {
        self.last_retrieval_stats
            .read()
            .map(|s| s.clone())
            .unwrap_or_default()
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
