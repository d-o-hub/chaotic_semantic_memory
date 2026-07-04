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
    /// Minimum score threshold (0.0-1.0).
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
mod tests {
    // Exact float comparisons for weight test assertions

    use super::*;

    #[test]
    fn test_compute_weights_short_query() {
        let (kw, sem) = compute_weights(1);
        assert!((kw - (0.9)).abs() < 1e-6);
        assert!((sem - (0.1)).abs() < 1e-6);

        let (kw, sem) = compute_weights(2);
        assert!((kw - (0.9)).abs() < 1e-6);
        assert!((sem - (0.1)).abs() < 1e-6);
    }

    #[test]
    fn test_compute_weights_medium_query() {
        let (kw, sem) = compute_weights(3);
        assert!((kw - (0.7)).abs() < 1e-6);
        assert!((sem - (0.3)).abs() < 1e-6);

        let (kw, sem) = compute_weights(4);
        assert!((kw - (0.7)).abs() < 1e-6);
        assert!((sem - (0.3)).abs() < 1e-6);
    }

    #[test]
    fn test_compute_weights_long_query() {
        let (kw, sem) = compute_weights(5);
        assert!((kw - (0.4)).abs() < 1e-6);
        assert!((sem - (0.6)).abs() < 1e-6);

        let (kw, sem) = compute_weights(8);
        assert!((kw - (0.4)).abs() < 1e-6);
        assert!((sem - (0.6)).abs() < 1e-6);
    }

    #[test]
    fn test_compute_weights_very_long_query() {
        let (kw, sem) = compute_weights(9);
        assert!((kw - (0.2)).abs() < 1e-6);
        assert!((sem - (0.8)).abs() < 1e-6);

        let (kw, sem) = compute_weights(100);
        assert!((kw - (0.2)).abs() < 1e-6);
        assert!((sem - (0.8)).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_scores_basic() {
        let scores = vec![
            ("a".to_string(), 10.0),
            ("b".to_string(), 15.0),
            ("c".to_string(), 20.0),
        ];
        let normalized = normalize_scores(&scores);

        assert!((normalized[0].1 - 0.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.5).abs() < 1e-6);
        assert!((normalized[2].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_scores_range_two() {
        let scores = vec![("a".to_string(), 2.0), ("b".to_string(), 0.0)];
        let normalized = normalize_scores(&scores);
        // (2-0)/2 = 1.0; (0-0)/2 = 0.0
        assert!((normalized[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_scores_empty() {
        let normalized = normalize_scores(&[]);
        assert!(normalized.is_empty());
    }

    #[test]
    fn test_normalize_scores_equal() {
        let scores = vec![("a".to_string(), 5.0), ("b".to_string(), 5.0)];
        let normalized = normalize_scores(&scores);

        // All equal scores should normalize to 1.0
        assert!((normalized[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized[1].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_merge_results_basic() {
        let bm25 = vec![("doc1".to_string(), 1.0), ("doc2".to_string(), 0.5)];
        let hdc = vec![("doc1".to_string(), 0.5), ("doc3".to_string(), 1.0)];

        let merged = merge_results(&bm25, &hdc, (0.5, 0.5));

        // doc1 appears in both
        assert!(merged.iter().any(|(id, _)| id == "doc1"));
        // doc2 only in BM25
        assert!(merged.iter().any(|(id, _)| id == "doc2"));
        // doc3 only in HDC
        assert!(merged.iter().any(|(id, _)| id == "doc3"));
    }

    #[test]
    fn test_merge_results_weighted() {
        let bm25 = vec![("doc1".to_string(), 1.0)];
        let hdc = vec![("doc1".to_string(), 1.0)];

        // With heavy keyword weight, BM25 should dominate
        let merged = merge_results(&bm25, &hdc, (0.9, 0.1));

        // doc1 should have combined score
        assert!(merged.iter().any(|(id, s)| id == "doc1" && *s > 0.0));
    }

    #[test]
    fn test_merge_results_empty() {
        let merged = merge_results(&[], &[], (0.5, 0.5));
        assert!(merged.is_empty());

        let merged = merge_results(&[("a".to_string(), 1.0)], &[], (0.5, 0.5));
        assert_eq!(merged.len(), 1);

        let merged = merge_results(&[], &[("a".to_string(), 1.0)], (0.5, 0.5));
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_exact_score_calculation() {
        // Use non-zero min values to catch replace - with + mutants.
        // Use weight != 0.5 to catch weight-related mutants.
        let weights = (0.6, 0.4);
        let bm25 = vec![
            ("d1".to_string(), 12.0),
            ("d2".to_string(), 2.0),
            ("d4".to_string(), 7.0),
        ];
        let hdc = vec![
            ("d1".to_string(), 1.2),
            ("d3".to_string(), 0.2),
            ("d4".to_string(), 0.7),
        ];

        let merged = merge_results(&bm25, &hdc, weights);

        // d1: 0.6 * 1.0 + 0.4 * 1.0 = 1.0
        let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
        assert!((d1_score - 1.0).abs() < 1e-6);

        // d2: 0.6 * 0.0 + 0.0 = 0.0
        let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
        assert!((d2_score - 0.0).abs() < 1e-6);

        // d3: 0.0 + 0.4 * 0.0 = 0.0
        let d3_score = merged.iter().find(|(id, _)| id == "d3").unwrap().1;
        assert!((d3_score - 0.0).abs() < 1e-6);

        // d4: 0.6 * 0.5 + 0.4 * 0.5 = 0.5
        let d4_score = merged.iter().find(|(id, _)| id == "d4").unwrap().1;
        assert!((d4_score - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_merge_results_equal_scores() {
        let bm25 = vec![("d1".to_string(), 5.0), ("d2".to_string(), 5.0)];
        let hdc = vec![("d1".to_string(), 0.5), ("d2".to_string(), 0.5)];
        let weights = (0.5, 0.5);
        let merged = merge_results(&bm25, &hdc, weights);
        // Each gets kw_weight (0.5) + sem_weight (0.5) = 1.0
        assert_eq!(merged.len(), 2);
        for (_, score) in merged {
            assert!((score - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_range_epsilon_boundary() {
        let epsilon = 1e-10;
        let scores = vec![("a".to_string(), epsilon), ("b".to_string(), 0.0)];
        let normalized = normalize_scores(&scores);
        // range = epsilon. epsilon < epsilon is false.
        assert!((normalized[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.0).abs() < 1e-6);

        let just_below = epsilon * 0.9;
        let scores_below = vec![("a".to_string(), just_below), ("b".to_string(), 0.0)];
        let normalized_below = normalize_scores(&scores_below);
        // range < epsilon is true.
        assert!((normalized_below[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized_below[1].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_merge_results_epsilon_boundary() {
        let epsilon = 1e-10;
        let weights = (0.5, 0.5);

        // Case 1: range exactly epsilon (should normalize)
        let bm25 = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
        let hdc = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
        let merged = merge_results(&bm25, &hdc, weights);
        let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
        let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
        assert!((d1_score - 1.0).abs() < 1e-6);
        assert!((d2_score - 0.0).abs() < 1e-6);

        // Case 2: range just below epsilon (should fallback to 1.0)
        let just_below = epsilon * 0.9;
        let bm25_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
        let hdc_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
        let merged_small = merge_results(&bm25_small, &hdc_small, weights);
        for (_, score) in merged_small {
            assert!((score - 1.0).abs() < 1e-6);
        }
    }

    fn config_with_threshold(min_score: f32) -> HybridConfig {
        HybridConfig {
            mode: HybridMode::Auto,
            min_score,
        }
    }

    #[test]
    fn test_hits_above_threshold() {
        let bm25 = vec![("doc_a".to_string(), 0.9)];
        let hdc = vec![("doc_a".to_string(), 0.8)];
        let config = config_with_threshold(0.5);
        let weights = (0.6, 0.4);
        let result = merge_results_checked(&bm25, &hdc, weights, &config, "test query");
        assert!(matches!(result, HybridResult::Hits(_)));
    }

    #[test]
    fn test_abstention_below_threshold() {
        let bm25 = vec![("doc_a".to_string(), 0.1)];
        let hdc = vec![("doc_a".to_string(), 0.05)];
        let config = config_with_threshold(0.5);
        let weights = (0.6, 0.4);
        let result = merge_results_checked(&bm25, &hdc, weights, &config, "unknown concept");
        match result {
            HybridResult::Abstained(a) => {
                assert_eq!(a.query, "unknown concept");
                assert!(a.best_score_seen < 0.5);
                assert!((a.min_score_threshold - 0.5).abs() < 1e-6);
            }
            HybridResult::Hits(_) => panic!("Expected abstention"),
        }
    }

    #[test]
    fn test_empty_results_produce_abstention() {
        let bm25: Vec<(String, f32)> = vec![];
        let hdc: Vec<(String, f32)> = vec![];
        let config = config_with_threshold(0.3);
        let weights = (0.5, 0.5);
        let result = merge_results_checked(&bm25, &hdc, weights, &config, "empty corpus query");
        assert!(matches!(result, HybridResult::Abstained(_)));
    }

    #[test]
    fn test_abstention_best_score_is_highest_seen() {
        let bm25 = vec![("doc_a".to_string(), 0.3), ("doc_b".to_string(), 0.1)];
        let hdc = vec![("doc_a".to_string(), 0.2)];
        let config = config_with_threshold(0.5);
        let weights = (0.6, 0.4);
        let result = merge_results_checked(&bm25, &hdc, weights, &config, "test");
        if let HybridResult::Abstained(a) = result {
            assert!(a.best_score_seen > 0.0 && a.best_score_seen < 0.5);
        } else {
            panic!("Expected abstention");
        }
    }
}
