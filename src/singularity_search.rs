//! Similarity search and cached retrieval methods for Singularity.
//!
//! Extracted from singularity.rs to satisfy the 500 LOC gate.

use std::sync::Arc;
use std::sync::atomic::Ordering;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::hyperdim::HVec10240;
use crate::singularity::{Singularity, similarity_cache_key, unix_now_ns};
use crate::singularity_retrieval::{CandidateSource, RetrievalStats, ScoredCandidateParams};

impl Singularity {
    /// Find similar concepts using cosine similarity
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, ns, query), fields(top_k = top_k)))]
    pub fn find_similar(&self, ns: &str, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        self.find_similar_arc(ns, query, top_k).as_ref().to_vec()
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_arc(&self, ns: &str, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
        self.find_similar_cached(ns, query, top_k)
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_cached(&self, ns: &str, query: &HVec10240, top_k: usize) -> Arc<[(String, f32)]> {
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
        let ns_state = self.get_namespace(ns).expect("Namespace checked by is_empty");

        let bypass_cache = top_k > self.config.max_cached_top_k;

        if !bypass_cache {
            let cache_key = similarity_cache_key(query, top_k);
            if let Ok(mut cache) = ns_state.query_cache.write() {
                if let Some(results) = cache.get(cache_key) {
                    ns_state.cache_metrics
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
                    return results;
                }
            }
            ns_state.cache_metrics
                .misses_total
                .fetch_add(1, Ordering::Relaxed);
        }

        // ADR-0068: Route through AnnIndex if it's not BruteForce.
        let index_stats = ns_state.index.stats();
        if index_stats.backend != "BruteForce" {
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
                            ns_state.cache_metrics
                                .evictions_total
                                .fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                return results_arc;
            }
        }

        // Generate candidates based on RetrievalConfig
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
        self.scored_candidate_retrieval(ns, ScoredCandidateParams {
            query,
            top_k,
            candidates,
            start_ns,
            cand_ns,
            source,
            bypass_cache,
        })
    }

    /// Find similar concepts with metadata filtering.
    pub fn find_similar_filtered(
        &self,
        ns: &str,
        query: &HVec10240,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
    ) -> Arc<[(String, f32)]> {
        let ns_state = self.get_namespace(ns).expect("Namespace must exist");
        let results = ns_state.index.search_filtered(query, top_k, filter, &ns_state.concepts).unwrap_or_default();
        Arc::from(results)
    }
}
