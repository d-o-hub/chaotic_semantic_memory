//! Retrieval optimization types and extension trait for Singularity.
//!
//! This module provides:
//! - `RetrievalStats`: Observability for retrieval operations
//! - `CandidateSource`: Where candidates came from
//! - `RetrievalConfig`: Configuration for candidate generation
//! - Extension trait for reduced-candidate retrieval

// Casts are intentional for retrieval similarity math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use crate::singularity::{Singularity, unix_now_ns};
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::HVec10240;

/// Statistics from the last retrieval operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievalStats {
    pub candidate_count: usize,
    pub scored_count: usize,
    pub fell_back_to_exact_scan: bool,
    pub candidate_ns: u64,
    pub scoring_ns: u64,
    pub best_score_seen: Option<f32>,
    /// ADR-0065: Filter selectivity ratio (matching_count / total_count)
    pub selectivity_ratio: f32,
    /// ADR-0065: Strategy used for filtered retrieval
    pub filter_strategy: Option<FilterStrategy>,
}

/// Source of candidates in reduced-candidate retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    Metadata,
    Graph,
    Bucket,
    ExactFallback,
}

/// Strategy used for filtered retrieval (ADR-0065).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterStrategy {
    /// Pre-filter candidates, then score (optimal for low selectivity)
    Pre,
    /// Generate bucket candidates, score, post-filter (optimal for medium selectivity)
    BucketPost,
    /// Full similarity scan, post-filter results (optimal for high selectivity)
    ScanPost,
}

/// Parameters for scored candidate retrieval.
pub struct ScoredCandidateParams<'a> {
    pub query: &'a HVec10240,
    pub top_k: usize,
    pub candidates: Vec<usize>,
    pub start_ns: u64,
    pub cand_ns: u64,
    pub source: CandidateSource,
    pub bypass_cache: bool,
}

/// Configuration for retrieval optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    pub max_candidates: usize,
    pub candidate_ratio_fallback: f32,
    pub graph_depth: u8,
    pub graph_fanout: usize,
    pub bucket_probe_width: usize,
    pub enable_graph_candidates: bool,
    pub enable_bucket_candidates: bool,
    /// Skip encode/LSH/graph when BM25 finds no token overlap.
    pub bm25_absence_short_circuit: bool,
    /// If first-stage top-1 HDC >= this, skip MMR + graph/bridge expansion.
    pub early_exit_hdc: f32,
    /// Maximum number of IDs Bridge Retrieval may append before extra scoring.
    pub bridge_expand_cap: usize,
}

/// Maximum allowed bucket probe width to prevent excessive memory usage.
const MAX_BUCKET_PROBE_WIDTH: usize = 16;

impl RetrievalConfig {
    pub fn validate(&self) -> Result<()> {
        if self.max_candidates == 0 {
            return Err(csm_core_lib::error::MemoryError::InvalidInput {
                field: "max_candidates".to_string(),
                reason: "max_candidates must be greater than 0".to_string(),
            });
        }
        if !(0.0..=1.0).contains(&self.early_exit_hdc) {
            return Err(csm_core_lib::error::MemoryError::InvalidInput {
                field: "early_exit_hdc".to_string(),
                reason: format!(
                    "early_exit_hdc must be in range [0.0, 1.0], got {}",
                    self.early_exit_hdc
                ),
            });
        }
        if self.bucket_probe_width > MAX_BUCKET_PROBE_WIDTH {
            return Err(csm_core_lib::error::MemoryError::InvalidInput {
                field: "bucket_probe_width".to_string(),
                reason: format!("bucket_probe_width exceeds {MAX_BUCKET_PROBE_WIDTH}"),
            });
        }
        Ok(())
    }

    /// Creates a [`RetrievalConfig`] tailored for a given query token count.
    #[must_use]
    pub fn for_token_count(n: usize) -> Self {
        let mut c = Self::default();
        match n {
            0..=2 => {
                c.max_candidates = 64;
                c.graph_depth = 0;
                c.enable_graph_candidates = false;
            }
            3..=4 => {
                c.max_candidates = 128;
                c.graph_depth = 1;
                c.graph_fanout = 4;
            }
            _ => {}
        }
        c
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_candidates: 256,
            candidate_ratio_fallback: 0.05,
            graph_depth: 1,
            graph_fanout: 8,
            bucket_probe_width: 2,
            enable_graph_candidates: true,
            enable_bucket_candidates: true,
            bm25_absence_short_circuit: true,
            early_exit_hdc: 0.92,
            bridge_expand_cap: 16,
        }
    }
}

impl Singularity {
    /// Set the retrieval configuration.
    pub fn set_retrieval_config(&mut self, config: RetrievalConfig) -> Result<()> {
        config.validate()?;
        self._retrieval_config = config;
        Ok(())
    }

    /// Get the retrieval configuration.
    /// Get statistics from the last retrieval operation.
    pub fn last_retrieval_stats(&self, ns: &str) -> RetrievalStats {
        self.get_namespace(ns)
            .and_then(|n| n.last_retrieval_stats.read().ok())
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Generate candidates by expanding the association graph.
    pub(crate) fn generate_graph_candidates(&self, ns: &str, query: &HVec10240) -> Vec<usize> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Vec::new();
        };
        let mut candidates = std::collections::HashSet::new();
        let results = self.exact_similarity_scan(ns, query, 1, unix_now_ns(), true);
        if let Some((seed_id, _)) = results.first() {
            let mut queue = VecDeque::new();
            queue.push_back((seed_id.as_str(), 0u8));
            candidates.insert(seed_id.as_str());

            while let Some((id, depth)) = queue.pop_front() {
                if depth >= self._retrieval_config.graph_depth {
                    continue;
                }
                if let Some(links) = ns_state.associations.get(id) {
                    let mut sorted_links: Vec<_> = links.iter().collect();
                    let fanout = self._retrieval_config.graph_fanout.min(sorted_links.len());
                    if fanout > 0 {
                        sorted_links
                            .select_nth_unstable_by(fanout - 1, |a, b| b.1.0.total_cmp(&a.1.0));
                        sorted_links.truncate(fanout);
                        sorted_links.sort_unstable_by(|a, b| b.1.0.total_cmp(&a.1.0));
                    }

                    for (neighbor_id, _) in sorted_links {
                        let neighbor_str = neighbor_id.as_str();
                        if !candidates.contains(neighbor_str) {
                            candidates.insert(neighbor_str);
                            queue.push_back((neighbor_str, depth + 1));
                        }
                    }
                }
            }
        }

        candidates
            .into_iter()
            .filter_map(|id| ns_state.id_to_index.get(id).copied())
            .collect()
    }

    /// Generate candidates by coarse bucketing.
    pub(crate) fn generate_bucket_candidates(&self, ns: &str, query: &HVec10240) -> Vec<usize> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Vec::new();
        };
        debug_assert!(self._retrieval_config.bucket_probe_width <= 127);
        let bucket_mask = (1u128 << self._retrieval_config.bucket_probe_width) - 1;
        let query_bucket = query.data[0] & bucket_mask;

        let filter = |(idx, vec): (usize, &HVec10240)| {
            if (vec.data[0] & bucket_mask) == query_bucket {
                Some(idx)
            } else {
                None
            }
        };

        // Algorithmic Optimization: Parallelize O(N) candidate generation via Rayon.
        // Reduces latency from O(N) to O(N/P) where P is the number of execution units.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        {
            ns_state
                .concept_vectors
                .par_iter()
                .enumerate()
                .filter_map(filter)
                .collect()
        }

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        {
            ns_state
                .concept_vectors
                .iter()
                .enumerate()
                .filter_map(filter)
                .collect()
        }
    }

    /// Perform exact similarity scan over all vectors.
    pub(crate) fn exact_similarity_scan(
        &self,
        ns: &str,
        query: &HVec10240,
        top_k: usize,
        start_ns: u64,
        bypass_cache: bool,
    ) -> Arc<[(String, f32)]> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };
        let scoring_start = unix_now_ns();

        // Algorithmic Optimization: Use integer Hamming distance for ranking to avoid floating-point
        // overhead and use a fused allocation to improve cache locality.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(usize, u32)> = ns_state
            .concept_vectors
            .par_iter()
            .enumerate()
            .with_min_len(128)
            .map(|(idx, v)| (idx, query.hamming_distance(v)))
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(usize, u32)> = ns_state
            .concept_vectors
            .iter()
            .enumerate()
            .map(|(idx, v)| (idx, query.hamming_distance(v)))
            .collect();

        let scoring_ns = unix_now_ns().saturating_sub(scoring_start);
        let scored_count = scores.len();

        // Sort by Hamming distance (ascending = more similar)
        if scored_count <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(idx, dist)| {
                // Defer cosine similarity calculation until the final top_k results
                let similarity = 1.0 - (dist as f32 / 5120.0);
                (ns_state.concept_indices[idx].clone(), similarity)
            })
            .collect();

        let best_score = results.first().map(|r| r.1);
        let results_arc = Arc::from(results);
        if !bypass_cache {
            if let Ok(mut cache) = ns_state.query_cache.write() {
                let cache_key = crate::singularity::similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    ns_state
                        .cache_metrics
                        .evictions_total
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        self.update_stats(
            ns,
            scored_count,
            scored_count,
            true,
            scoring_start.saturating_sub(start_ns),
            scoring_ns,
            best_score,
            1.0,  // Full scan means 100% selectivity for unfiltered
            None, // No filter strategy for unfiltered
        );
        results_arc
    }

    /// Score a subset of candidates for reduced-candidate retrieval.
    pub(crate) fn scored_candidate_retrieval(
        &self,
        ns: &str,
        params: ScoredCandidateParams,
    ) -> Arc<[(String, f32)]> {
        self.scored_candidate_retrieval_with_stats(ns, params, 0.0, None)
    }

    /// Update retrieval statistics.
    #[allow(clippy::too_many_arguments)]
    fn update_stats(
        &self,
        ns: &str,
        candidates: usize,
        scored: usize,
        fallback: bool,
        cand_ns: u64,
        score_ns: u64,
        best_score: Option<f32>,
        selectivity: f32,
        strategy: Option<FilterStrategy>,
    ) {
        if let Some(ns_state) = self.get_namespace(ns) {
            let stats = RetrievalStats {
                candidate_count: candidates,
                scored_count: scored,
                fell_back_to_exact_scan: fallback,
                candidate_ns: cand_ns,
                scoring_ns: score_ns,
                best_score_seen: best_score,
                selectivity_ratio: selectivity,
                filter_strategy: strategy,
            };
            if let Ok(mut s) = ns_state.last_retrieval_stats.write() {
                *s = stats;
            }
        }
    }

    /// Score candidates with explicit selectivity stats (ADR-0065).
    pub(crate) fn scored_candidate_retrieval_with_stats(
        &self,
        ns: &str,
        params: ScoredCandidateParams,
        selectivity: f32,
        strategy: Option<FilterStrategy>,
    ) -> Arc<[(String, f32)]> {
        let Some(ns_state) = self.get_namespace(ns) else {
            return Arc::from(Vec::new());
        };
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

        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(usize, u32)> = candidates
            .into_par_iter()
            .map(|idx| (idx, query.hamming_distance(&ns_state.concept_vectors[idx])))
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(usize, u32)> = candidates
            .into_iter()
            .map(|idx| (idx, query.hamming_distance(&ns_state.concept_vectors[idx])))
            .collect();

        let scoring_ns = unix_now_ns().saturating_sub(scoring_start);
        let scored_count = scores.len();

        if scores.len() <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(idx, dist)| {
                let similarity = 1.0 - (dist as f32 / 5120.0);
                (ns_state.concept_indices[idx].clone(), similarity)
            })
            .collect();

        let best_score = results.first().map(|r| r.1);
        let results_arc = Arc::from(results);
        if !bypass_cache {
            if let Ok(mut cache) = ns_state.query_cache.write() {
                let cache_key = crate::singularity::similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    ns_state
                        .cache_metrics
                        .evictions_total
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        self.update_stats(
            ns,
            candidate_count,
            scored_count,
            false,
            cand_ns,
            scoring_ns,
            best_score,
            selectivity,
            strategy,
        );

        results_arc
    }
}

include!("singularity_retrieval_tests.rs");

