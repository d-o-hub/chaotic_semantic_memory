//! Similarity search and cached retrieval methods for Singularity.
//!
//! Extracted from singularity.rs to satisfy the 500 LOC gate.

use std::sync::Arc;
use std::sync::atomic::Ordering;

#[cfg(not(target_arch = "wasm32"))]
use tracing::instrument;

use crate::singularity::{Singularity, similarity_cache_key, unix_now_ns};
use crate::singularity_retrieval::{
    CandidateSource, FilterStrategy, RetrievalStats, ScoredCandidateParams,
};
use crate::singularity_state::NamespaceState;
use csm_core_lib::hyperdim::HVec10240;

// ── Helper functions for find_similar_cached ──────────────────────────
// Extracted to reduce cyclomatic complexity (Deepsource).

/// Try to retrieve results from the similarity cache.
/// Returns `Some(results)` on cache hit, `None` on miss or cache bypass.
fn try_cache_lookup(
    ns_state: &NamespaceState,
    query: &HVec10240,
    top_k: usize,
    bypass_cache: bool,
    start_ns: u64,
) -> Option<Arc<[(String, f32)]>> {
    if bypass_cache {
        return None;
    }

    let cache_key = similarity_cache_key(query, top_k);
    if let Ok(mut cache) = ns_state.query_cache.write()
        && let Some(results) = cache.get(cache_key)
    {
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
    ns_state
        .cache_metrics
        .misses_total
        .fetch_add(1, Ordering::Relaxed);
    None
}

/// Try to retrieve results from the ANN index.
/// Returns `Some(results)` on ANN hit, `None` for BruteForce backend or
/// ANN search failure (falls through to exact scan).
fn try_ann_lookup(
    ns_state: &NamespaceState,
    query: &HVec10240,
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
        let best_score = results_arc.first().map(|r| r.1);

        if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
            s.scored_count = results_arc.len();
            s.candidate_count = index_stats.count;
            s.scoring_ns = unix_now_ns().saturating_sub(start_ns);
            s.best_score_seen = best_score;
        }

        if !bypass_cache && let Ok(mut cache) = ns_state.query_cache.write() {
            let cache_key = similarity_cache_key(query, top_k);
            if cache.put(cache_key, Arc::clone(&results_arc)) {
                ns_state
                    .cache_metrics
                    .evictions_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        return Some(results_arc);
    }
    None
}

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

    /// Find similar concepts, returning cached results as `Arc<[_]>`.
    ///
    /// Retrieval pipeline:
    /// 1. Cache lookup (bypassed when `top_k > max_cached_top_k`)
    /// 2. ANN index lookup (skipped for BruteForce backend)
    /// 3. Candidate generation (graph → bucket → exact scan fallback)
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
            if let Some(ns_state) = self.get_namespace(ns)
                && let Ok(mut s) = ns_state.last_retrieval_stats.write()
            {
                *s = stats;
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

        if candidates.len() > self._retrieval_config.max_candidates {
            candidates.truncate(self._retrieval_config.max_candidates);
        }

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

    /// Search active concepts and apply metadata filtering before top-k selection.
    ///
    /// The common no-expiration path retains the existing ANN/cache strategies.
    /// Once any concept expires, an eligibility scan avoids letting expired rows
    /// consume result slots and bypasses cached results that may have crossed TTL.
    pub fn find_similar_active(
        &self,
        ns: &str,
        query: &HVec10240,
        top_k: usize,
        now: u64,
        filter: Option<&crate::metadata_filter::MetadataFilter>,
    ) -> Arc<[(String, f32)]> {
        if top_k == 0 {
            return Arc::from(Vec::new());
        }
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };
        let has_expired = ns_state
            .concepts
            .values()
            .any(|concept| concept.expires_at.is_some_and(|expires| expires <= now));
        if !has_expired {
            return filter.map_or_else(
                || self.find_similar_cached(ns, query, top_k),
                |predicate| self.find_similar_filtered(ns, query, top_k, predicate),
            );
        }

        let scoring_start = unix_now_ns();
        let total = ns_state.concept_indices.len();
        let mut scores = Vec::with_capacity(total);
        for (index, id) in ns_state.concept_indices.iter().enumerate() {
            let Some(concept) = ns_state.concepts.get(id) else {
                continue;
            };
            if concept.expires_at.is_some_and(|expires| expires <= now)
                || filter.is_some_and(|predicate| !predicate.matches(&concept.metadata))
            {
                continue;
            }
            scores.push((
                index,
                query.hamming_distance(&ns_state.concept_vectors[index]),
            ));
        }
        let scored_count = scores.len();
        if scores.len() > top_k {
            scores.select_nth_unstable_by(top_k - 1, |left, right| left.1.cmp(&right.1));
            scores.truncate(top_k);
        }
        scores.sort_unstable_by_key(|(_, distance)| *distance);
        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(index, distance)| {
                // HVec10240 distances are at most 10,240, exactly representable
                // through u16 and f32. Keep the conversion checked so a future
                // wider vector does not silently truncate a distance.
                let score = u16::try_from(distance)
                    .map_or(0.0, |distance| 1.0 - (f32::from(distance) / 5120.0));
                (ns_state.concept_indices[index].clone(), score)
            })
            .collect();
        let best_score = results.first().map(|result| result.1);
        let results: Arc<[(String, f32)]> = Arc::from(results);
        #[allow(clippy::cast_precision_loss)]
        let selectivity = if total > 0 {
            scored_count as f32 / total as f32
        } else {
            0.0
        };
        let stats = RetrievalStats {
            candidate_count: total,
            scored_count,
            fell_back_to_exact_scan: true,
            scoring_ns: unix_now_ns().saturating_sub(scoring_start),
            best_score_seen: best_score,
            selectivity_ratio: selectivity,
            filter_strategy: filter.map(|_| FilterStrategy::Pre),
            ..Default::default()
        };
        if let Ok(mut last) = ns_state.last_retrieval_stats.write() {
            *last = stats;
        }
        results
    }
    /// Find similar concepts with metadata filtering.
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
