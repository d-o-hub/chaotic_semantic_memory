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

        // ADR-0068: Route through AnnIndex if it's not BruteForce.
        // We check stats to see backend name as we don't want to bypass
        // the specialized heuristic generation (graph/bucket) if we are in BruteForce mode
        // which IS the fallback.
        let index_stats = self.index.stats();
        if index_stats.backend != "BruteForce" {
            if let Ok(results) = self.index.search(query, top_k) {
                // #10: Guard: when results are empty but concepts are not, fall back to exact scan.
                if results.is_empty() && !self.concepts.is_empty() {
                    // Fall through to heuristic generation or exact scan
                } else {
                    let results_arc: Arc<[(String, f32)]> = Arc::from(results);

                    // ADR-0068: Update stats for ANN search
                    if let Ok(mut s) = self.last_retrieval_stats.write() {
                        s.scored_count = results_arc.len();
                        s.candidate_count = index_stats.count;
                        s.scoring_ns = unix_now_ns().saturating_sub(start_ns);
                    }

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
                    return results_arc;
                }
            }
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
            // BruteForce backend fallback
            if let Ok(results) = self.index.search(query, top_k) {
                let results_arc: Arc<[(String, f32)]> = Arc::from(results);

                if let Ok(mut s) = self.last_retrieval_stats.write() {
                    s.scored_count = results_arc.len();
                    s.candidate_count = index_stats.count;
                    s.scoring_ns = unix_now_ns().saturating_sub(start_ns);
                    s.fell_back_to_exact_scan = true;
                }

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
                return results_arc;
            }
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
}
