//! Similarity search and cached retrieval methods for Singularity.
//!
//! Extracted from singularity.rs to satisfy the 500 LOC gate.

use std::sync::Arc;
use std::sync::atomic::Ordering;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::hyperdim::HVec10240;
use crate::singularity::{Singularity, similarity_cache_key, unix_now_ns};
use crate::singularity_retrieval::{
    CandidateSource, FilterStrategy, RetrievalStats, ScoredCandidateParams,
};

impl Singularity {
    /// Find similar concepts using cosine similarity
    #[cfg_attr(not(target_arch = "wasm32"), instrument(skip(self, ns, query), fields(top_k = top_k)))]
    pub fn find_similar(&self, ns: &str, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
        self.find_similar_arc(ns, query, top_k).as_ref().to_vec()
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_arc(
        &self,
        ns: &str,
        query: &HVec10240,
        top_k: usize,
    ) -> Arc<[(String, f32)]> {
        self.find_similar_cached(ns, query, top_k)
    }

    /// Find similar concepts and return cached results as `Arc<[_]>`.
    pub fn find_similar_cached(
        &self,
        ns: &str,
        query: &HVec10240,
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
        let ns_state = self
            .get_namespace(ns)
            .expect("Namespace checked by is_empty");

        let bypass_cache = top_k > self.config.max_cached_top_k;

        if let Some(results) = self.try_cache_hit(ns_state, query, top_k, bypass_cache, start_ns) {
            return results;
        }

        // ADR-0068: Route through AnnIndex if it's not BruteForce.
        if let Some(results) = self.try_ann_search(ns_state, query, top_k, bypass_cache, start_ns) {
            return results;
        }

        // Generate candidates based on RetrievalConfig
        let (candidates, source, _cand_ns) = self.generate_candidates(ns, query);

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
                cand_ns: 0,
                source,
                bypass_cache,
            },
        )
    }

    fn try_cache_hit(
        &self,
        ns_state: &crate::singularity_state::NamespaceState,
        query: &HVec10240,
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

    fn try_ann_search(
        &self,
        ns_state: &crate::singularity_state::NamespaceState,
        query: &HVec10240,
        top_k: usize,
        bypass_cache: bool,
        start_ns: u64,
    ) -> Option<Arc<[(String, f32)]>> {
        let index_stats = ns_state.index.stats();
        if index_stats.backend == "BruteForce" {
            return None;
        }
        let results = ns_state.index.search(query, top_k).ok()?;
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
        Some(results_arc)
    }

    fn generate_candidates(
        &self,
        ns: &str,
        query: &HVec10240,
    ) -> (Vec<usize>, CandidateSource, u64) {
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
        (candidates, source, cand_ns)
    }

    /// Find similar concepts with metadata filtering (ADR-0065: selectivity-aware).
    #[allow(clippy::cast_precision_loss)]
    pub fn find_similar_filtered(
        &self,
        ns: &str,
        query: &HVec10240,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
    ) -> Arc<[(String, f32)]> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };
        let total = ns_state.concepts.len();
        if total == 0 {
            return Arc::from(Vec::new());
        }

        // Compute selectivity: fraction of concepts matching the filter
        let matching_count = ns_state
            .concepts
            .values()
            .filter(|c| filter.matches(&c.metadata))
            .count();
        let selectivity = matching_count as f32 / total as f32;

        // ADR-0065: Choose filter strategy based on selectivity
        const SMALL_DATASET_THRESHOLD: usize = 20;
        const LOW_SELECTIVITY_THRESHOLD: f32 = 0.3;
        const HIGH_SELECTIVITY_THRESHOLD: f32 = 0.8;

        let strategy =
            if total < SMALL_DATASET_THRESHOLD || selectivity <= LOW_SELECTIVITY_THRESHOLD {
                FilterStrategy::Pre
            } else if selectivity <= HIGH_SELECTIVITY_THRESHOLD {
                FilterStrategy::BucketPost
            } else {
                FilterStrategy::ScanPost
            };

        let start_ns = crate::singularity::unix_now_ns();

        // Execute the filtered search using the index
        let results = ns_state
            .index
            .search_filtered(query, top_k, filter, &ns_state.concepts)
            .unwrap_or_default();

        // Record retrieval stats with selectivity and strategy
        if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
            *s = crate::singularity_retrieval::RetrievalStats {
                candidate_count: total,
                scored_count: results.len(),
                fell_back_to_exact_scan: true,
                candidate_ns: 0,
                scoring_ns: crate::singularity::unix_now_ns().saturating_sub(start_ns),
                selectivity_ratio: selectivity,
                filter_strategy: Some(strategy),
            };
        }

        Arc::from(results)
    }
}
