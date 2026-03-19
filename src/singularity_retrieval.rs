//! Retrieval optimization types and extension trait for Singularity.
//!
//! This module provides:
//! - `RetrievalStats`: Observability for retrieval operations
//! - `CandidateSource`: Where candidates came from
//! - `RetrievalConfig`: Configuration for candidate generation
//! - Extension trait for reduced-candidate retrieval

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::hyperdim::HVec10240;
use crate::singularity::{Singularity, unix_now_ns};

/// Statistics from the last retrieval operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetrievalStats {
    pub candidate_count: usize,
    pub scored_count: usize,
    pub fell_back_to_exact_scan: bool,
    pub candidate_ns: u64,
    pub scoring_ns: u64,
}

/// Source of candidates in reduced-candidate retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSource {
    Metadata,
    Graph,
    Bucket,
    ExactFallback,
}

/// Parameters for scored candidate retrieval.
pub(crate) struct ScoredCandidateParams<'a> {
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

impl Singularity {
    /// Set the retrieval configuration.
    pub fn set_retrieval_config(&mut self, config: RetrievalConfig) {
        self.retrieval_config = config;
    }

    /// Get the retrieval configuration.
    pub fn retrieval_config(&self) -> &RetrievalConfig {
        &self.retrieval_config
    }

    /// Get statistics from the last retrieval operation.
    pub fn last_retrieval_stats(&self) -> RetrievalStats {
        self.last_retrieval_stats
            .read()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Generate candidates by expanding the association graph.
    pub(crate) fn generate_graph_candidates(&self, query: &HVec10240) -> Vec<usize> {
        let mut candidates = std::collections::HashSet::new();
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

    /// Generate candidates by coarse bucketing.
    pub(crate) fn generate_bucket_candidates(&self, query: &HVec10240) -> Vec<usize> {
        let bucket_count = 1 << self.retrieval_config.bucket_probe_width;
        let query_bucket = (query.data[0] % bucket_count as u128) as usize;

        self.concept_vectors
            .iter()
            .enumerate()
            .filter_map(|(idx, vec)| {
                let vec_bucket = (vec.data[0] % bucket_count as u128) as usize;
                if vec_bucket == query_bucket {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Perform exact similarity scan over all vectors.
    pub(crate) fn exact_similarity_scan(
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
                let cache_key = crate::singularity::similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    self.cache_metrics
                        .evictions_total
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

    /// Score a subset of candidates for reduced-candidate retrieval.
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
                let cache_key = crate::singularity::similarity_cache_key(query, top_k);
                if cache.put(cache_key, Arc::clone(&results_arc)) {
                    self.cache_metrics
                        .evictions_total
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        self.update_stats(candidate_count, scored_count, false, cand_ns, scoring_ns);

        results_arc
    }

    /// Update retrieval statistics.
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
}
