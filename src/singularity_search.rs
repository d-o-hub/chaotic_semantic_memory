//! Similarity search and cached retrieval methods for Singularity.
//!
//! Extracted from singularity.rs to satisfy the 500 LOC gate.

use std::sync::Arc;
use std::sync::atomic::Ordering;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::hyperdim::{HVec10240, Hypervector};
use crate::singularity::{Singularity, similarity_cache_key, unix_now_ns};
use crate::singularity_retrieval::{
    CandidateSource, FilterStrategy, RetrievalStats, ScoredCandidateParams,
};
use crate::singularity_state::NamespaceState;

// ── Helper functions for find_similar_cached ──────────────────────────
// Extracted to reduce cyclomatic complexity (Deepsource).

/// Try to retrieve results from the similarity cache.
/// Returns `Some(results)` on cache hit, `None` on miss or cache bypass.
fn try_cache_lookup<H: Hypervector>(
    ns_state: &NamespaceState<H>,
    query: &H,
    top_k: usize,
    bypass_cache: bool,
    start_ns: u64,
) -> Option<Arc<[(String, f32)]>> {
    if bypass_cache {
        return None;
    }

    let cache_key = similarity_cache_key(query, top_k);
    if let Ok(mut cache) = ns_state.query_cache.write() {
        if let Some(results) = cache.get(cache_key) {
            ns_state
                .cache_metrics
                .hits_total
                .fetch_add(1, Ordering::Relaxed);
            let stats = RetrievalStats {
                candidate_count: results.len(),
                scored_count: 0,
                scoring_ns: unix_now_ns().saturating_sub(start_ns),
                ..Default::default()
            };
            if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
                *s = stats;
            }
            return Some(results);
        }
    }
    ns_state
        .cache_metrics
        .misses_total
        .fetch_add(1, Ordering::Relaxed);
    None
}

/// Try to retrieve results from the ANN index.
/// Returns `Some(results)` on ANN hit, `None` for BruteForce backend or
/// ANN search failure (falls through to exact scan).
fn try_ann_lookup<H: Hypervector>(
    ns_state: &NamespaceState<H>,
    query: &H,
    top_k: usize,
    bypass_cache: bool,
    start_ns: u64,
) -> Option<Arc<[(String, f32)]>> {
    let index_stats = ns_state.index.stats();
    if index_stats.backend == "BruteForce" {
        return None;
    }

    if let Ok(results) = ns_state.index.search(query, top_k) {
        let results_arc: Arc<[(String, f32)]> = Arc::from(results);

        if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
            s.scored_count = results_arc.len();
            s.candidate_count = index_stats.count;
            s.scoring_ns = unix_now_ns().saturating_sub(start_ns);
        }

        if !bypass_cache {
            if let Ok(mut cache) = ns_state.query_cache.write() {
                let cache_key = similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    ns_state
                        .cache_metrics
                        .evictions_total
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        return Some(results_arc);
    }
    None
}

impl<H: Hypervector + \'static> Singularity<H> {
    /// Find similar concepts using cosine similarity
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, ns, query), fields(top_k = top_k)))]
    pub fn find_similar(&self, ns: &str, query: &H, top_k: usize) -> Vec<(String, f32)> {
        self.find_similar_arc(ns, query, top_k).as_ref().to_vec()
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_arc(
        &self,
        ns: &str,
        query: &H,
        top_k: usize,
    ) -> Arc<[(String, f32)]> {
        self.find_similar_cached(ns, query, top_k)
    }

    /// Find similar concepts, returning cached results as `Arc<[_]>`.
    ///
    /// Retrieval pipeline:
    /// 1. Cache lookup (bypassed when `top_k > max_cached_top_k`)
    /// 2. ANN index lookup (skipped for BruteForce backend)
    /// 3. Candidate generation (graph → bucket → exact scan fallback)
    pub fn find_similar_cached(
        &self,
        ns: &str,
        query: &H,
        top_k: usize,
    ) -> Arc<[(String, f32)]> {
        let start_ns = unix_now_ns();
        if top_k == 0 || self.is_empty(ns) {
            let stats = RetrievalStats {
                fell_back_to_exact_scan: true,
                ..Default::default()
            };
            if let Some(ns_state) = self.get_namespace(ns) {
                if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
                    *s = stats;
                }
            }
            return Arc::from(Vec::new());
        }
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };

        let bypass_cache = top_k > self.config.max_cached_top_k;

        // Step 1: Cache lookup
        if let Some(results) = try_cache_lookup(ns_state, query, top_k, bypass_cache, start_ns) {
            return results;
        }

        // Step 2: ANN index lookup (ADR-0068)
        if let Some(results) = try_ann_lookup(ns_state, query, top_k, bypass_cache, start_ns) {
            return results;
        }

        // Step 3: Candidate generation based on RetrievalConfig
        let candidate_start = unix_now_ns();
        let mut candidates = Vec::new();
        let mut source = CandidateSource::ExactFallback;

        if self._retrieval_config.enable_graph_candidates {
            candidates = self.generate_graph_candidates(ns, query);
            if !candidates.is_empty() {
                source = CandidateSource::Graph;
            }
        }
        if candidates.is_empty() && self._retrieval_config.enable_bucket_candidates {
            candidates = self.generate_bucket_candidates(ns, query);
            if !candidates.is_empty() {
                source = CandidateSource::Bucket;
            }
        }

        let cand_ns = unix_now_ns().saturating_sub(candidate_start);

        if candidates.is_empty() {
            return self.exact_similarity_scan(ns, query, top_k, start_ns, bypass_cache);
        }

        // Reduced-candidate path
        self.scored_candidate_retrieval(
            ns,
            ScoredCandidateParams {
                query,
                top_k,
                candidates,
                start_ns,
                cand_ns,
                source,
                bypass_cache,
            },
        )
    }

    /// Find similar concepts with metadata filtering.
    pub fn find_similar_filtered(
        &self,
        ns: &str,
        query: &H,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
    ) -> Arc<[(String, f32)]> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };

        let total = ns_state.concepts.len();
        let matching = ns_state
            .concepts
            .values()
            .filter(|c| filter.matches(&c.metadata))
            .count();

        #[allow(clippy::cast_precision_loss)] // concept counts fit in f32 for selectivity
        let selectivity = if total > 0 {
            matching as f32 / total as f32
        } else {
            0.0
        };

        // ADR-0065: Select strategy based on selectivity
        #[allow(clippy::cast_precision_loss)] // concept counts fit in f32 for selectivity
        let strategy = if total < 20 || selectivity < 0.3 {
            Some(FilterStrategy::Pre)
        } else if selectivity <= 0.8 {
            Some(FilterStrategy::BucketPost)
        } else {
            Some(FilterStrategy::ScanPost)
        };

        let results = ns_state
            .index
            .search_filtered(query, top_k, filter, &ns_state.concepts)
            .unwrap_or_default();

        let results_arc: Arc<[(String, f32)]> = Arc::from(results);

        let stats = RetrievalStats {
            candidate_count: matching,
            scored_count: results_arc.len(),
            selectivity_ratio: selectivity,
            filter_strategy: strategy,
            ..Default::default()
        };
        if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
            *s = stats;
        }

        results_arc
    }
}
