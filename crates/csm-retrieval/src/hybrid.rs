//! Hybrid retrieval combining BM25 and HDC scores.
//!
//! Provides query-length-dependent weighting between keyword (BM25) and
//! semantic (HDC) search results.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Structured signal emitted when hybrid retrieval cannot find results
/// above the configured confidence threshold across all tiers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievalAbstention {
    /// The query that failed to produce results
    pub query: String,
    /// The threshold that was not met
    pub min_score_threshold: f32,
    /// Highest score actually seen across all tiers (for diagnostics)
    pub best_score_seen: f32,
    /// Which retrieval modes were attempted before abstaining
    pub attempted_modes: Vec<String>,
    /// UTC timestamp of the abstention event
    pub timestamp: DateTime<Utc>,
}

/// The result of a hybrid retrieval attempt.
/// Replaces bare `Vec<(String, f32)>` at the public API boundary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HybridResult {
    /// One or more results above the min_score threshold.
    Hits(Vec<(String, f32)>),
    /// All tiers returned results below min_score: the system abstains.
    Abstained(RetrievalAbstention),
}

/// Compute query-length-dependent weights for hybrid retrieval.
///
/// Returns (keyword_weight, semantic_weight) based on token count.
///
/// | Query Tokens | Keyword | Semantic | Rationale |
/// |-------------|---------|----------|-----------|
/// | 1-2 | 0.9 | 0.1 | Exact match dominates |
/// | 3-4 | 0.7 | 0.3 | Keyword still strong |
/// | 5-8 | 0.4 | 0.6 | Semantic takes over |
/// | 9+ | 0.2 | 0.8 | Full semantic mode |
#[allow(dead_code)]
pub const fn compute_weights(token_count: usize) -> (f32, f32) {
    match token_count {
        1..=2 => (0.9, 0.1),
        3..=4 => (0.7, 0.3),
        5..=8 => (0.4, 0.6),
        _ => (0.2, 0.8),
    }
}

/// Normalize scores to [0, 1] range using min-max normalization.
///
/// If all scores are equal, returns 1.0 for all.
pub fn normalize_scores(scores: &[(String, f32)]) -> Vec<(String, f32)> {
    if scores.is_empty() {
        return Vec::new();
    }

    // Optimization: Single-pass min-max calculation
    let (min, max) = scores
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), (_, s)| {
            (min.min(*s), max.max(*s))
        });

    let range = max - min;
    let epsilon = 1e-10;

    if range < epsilon {
        return scores.iter().map(|(id, _)| (id.clone(), 1.0)).collect();
    }

    // Optimization: Use inverse range multiplication instead of division in loop
    let inv_range = 1.0 / range;
    scores
        .iter()
        .map(|(id, score)| {
            let normalized = (score - min) * inv_range;
            (id.clone(), normalized)
        })
        .collect()
}

/// Merge BM25 and HDC results with given weights.
///
/// Takes two result sets (from BM25 and HDC), normalizes scores,
/// and combines them using weighted sum.
///
/// Duplicate IDs are merged by taking the maximum combined score.
pub fn merge_results(
    bm25_results: &[(String, f32)],
    hdc_results: &[(String, f32)],
    weights: (f32, f32),
) -> Vec<(String, f32)> {
    let (kw_weight, sem_weight) = weights;

    // Optimization: Pre-allocate map to avoid redundant re-hashes and re-allocs.
    // Capacity is at most the sum of both result sets.
    // Use &str as key to avoid redundant String clones during accumulation.
    let mut combined: HashMap<&str, f32> =
        HashMap::with_capacity(bm25_results.len() + hdc_results.len());

    // Optimization: Eliminate intermediate Vec allocations by merging and
    // normalizing in a single pass.

    if !bm25_results.is_empty() {
        let (min, max) = bm25_results
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), (_, s)| {
                (min.min(*s), max.max(*s))
            });
        let range = max - min;
        if range < 1e-10 {
            for (id, _) in bm25_results {
                combined.insert(id.as_str(), kw_weight);
            }
        } else {
            let inv_range = 1.0 / range;
            for (id, score) in bm25_results {
                combined.insert(id.as_str(), kw_weight * (score - min) * inv_range);
            }
        }
    }

    if !hdc_results.is_empty() {
        let (min, max) = hdc_results
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), (_, s)| {
                (min.min(*s), max.max(*s))
            });
        let range = max - min;
        if range < 1e-10 {
            for (id, _) in hdc_results {
                // Optimization: Use Entry API to combine lookup and insertion, avoiding redundant hashing.
                combined
                    .entry(id.as_str())
                    .and_modify(|s| *s += sem_weight)
                    .or_insert(sem_weight);
            }
        } else {
            let inv_range = 1.0 / range;
            for (id, score) in hdc_results {
                let weighted_norm = sem_weight * (score - min) * inv_range;
                combined
                    .entry(id.as_str())
                    .and_modify(|s| *s += weighted_norm)
                    .or_insert(weighted_norm);
            }
        }
    }

    // Sort by combined score descending and clone IDs only once for the final results.
    let mut results: Vec<(String, f32)> = combined
        .into_iter()
        .map(|(id, score)| (id.to_string(), score))
        .collect();
    results.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

    results
}

/// Merge BM25 and HDC results with query-length-dependent weights.
/// Returns `HybridResult::Abstained` if no merged result meets `config.min_score`.
///
/// NOTE: This function applies the `min_score` threshold BEFORE normalization
/// to properly implement Agentic Abstention (arXiv:2606.28733).
pub fn merge_results_checked(
    bm25_results: &[(String, f32)],
    hdc_results: &[(String, f32)],
    weights: (f32, f32),
    config: &HybridConfig,
    query: &str,
) -> HybridResult {
    // 1. First merge RAW scores with weights
    let (kw_weight, sem_weight) = weights;
    let mut combined: HashMap<&str, f32> =
        HashMap::with_capacity(bm25_results.len() + hdc_results.len());

    for (id, score) in bm25_results {
        combined.insert(id.as_str(), kw_weight * score);
    }

    for (id, score) in hdc_results {
        combined
            .entry(id.as_str())
            .and_modify(|s| *s += sem_weight * score)
            .or_insert(sem_weight * score);
    }

    // 2. Filter by absolute threshold BEFORE normalization
    let above_threshold: Vec<(String, f32)> = combined
        .iter()
        .filter(|&(_, &score)| score >= config.min_score)
        .map(|(&id, &score)| (id.to_string(), score))
        .collect();

    if above_threshold.is_empty() {
        let best_score_seen = combined.values().fold(0.0_f32, |m, &s| m.max(s));

        HybridResult::Abstained(RetrievalAbstention {
            query: query.to_string(),
            min_score_threshold: config.min_score,
            best_score_seen,
            attempted_modes: vec![format!("{:?}", config.mode)],
            timestamp: Utc::now(),
        })
    } else {
        // 3. Apply normalization to the final result set for consistent ranking/display
        let normalized = normalize_scores(&above_threshold);
        let mut results = normalized;
        results.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        HybridResult::Hits(results)
    }
}

/// Hybrid retrieval mode.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HybridMode {
    /// Auto-weight by query length (default).
    #[default]
    Auto,
    /// Force semantic-only (HDC).
    SemanticOnly,
    /// Force keyword-only (BM25).
    KeywordOnly,
    /// Custom weight override.
    Custom(f32),
}

/// Configuration for hybrid retrieval.
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Hybrid mode.
    pub mode: HybridMode,
    /// Minimum score threshold applied by `merge_results_checked` to weighted
    /// raw scores (`kw_weight*bm25 + sem_weight*hdc`) before normalization.
    /// Note this is a raw-score cutoff, not a normalized [0,1] confidence.
    pub min_score: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            mode: HybridMode::Auto,
            min_score: 0.0,
        }
    }
}
#[cfg(test)]
#[path = "hybrid/tests.rs"]
mod tests;
