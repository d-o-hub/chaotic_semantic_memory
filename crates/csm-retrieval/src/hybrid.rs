//! Hybrid retrieval combining BM25 and HDC scores.
//!
//! Provides query-length-dependent weighting between keyword (BM25) and
//! semantic (HDC) search results.

use std::collections::HashMap;

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
    let (min, max) = scores.iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min, max), (_, s)| (min.min(*s), max.max(*s)),
    );

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
    let mut combined: HashMap<String, f32> =
        HashMap::with_capacity(bm25_results.len() + hdc_results.len());

    // Optimization: Eliminate intermediate Vec allocations by merging and
    // normalizing in a single pass.

    if !bm25_results.is_empty() {
        let (min, max) = bm25_results.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(min, max), (_, s)| (min.min(*s), max.max(*s)),
        );
        let range = max - min;
        if range < 1e-10 {
            for (id, _) in bm25_results {
                combined.insert(id.clone(), kw_weight);
            }
        } else {
            let inv_range = 1.0 / range;
            for (id, score) in bm25_results {
                combined.insert(id.clone(), kw_weight * (score - min) * inv_range);
            }
        }
    }

    if !hdc_results.is_empty() {
        let (min, max) = hdc_results.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(min, max), (_, s)| (min.min(*s), max.max(*s)),
        );
        let range = max - min;
        if range < 1e-10 {
            for (id, _) in hdc_results {
                combined
                    .entry(id.clone())
                    .and_modify(|s| *s += sem_weight)
                    .or_insert(sem_weight);
            }
        } else {
            let inv_range = 1.0 / range;
            for (id, score) in hdc_results {
                let weighted_norm = sem_weight * (score - min) * inv_range;
                combined
                    .entry(id.clone())
                    .and_modify(|s| *s += weighted_norm)
                    .or_insert(weighted_norm);
            }
        }
    }

    // Sort by combined score descending
    let mut results: Vec<(String, f32)> = combined.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
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
            ("a".to_string(), 0.0),
            ("b".to_string(), 0.5),
            ("c".to_string(), 1.0),
        ];
        let normalized = normalize_scores(&scores);

        assert!((normalized[0].1 - 0.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.5).abs() < 1e-6);
        assert!((normalized[2].1 - 1.0).abs() < 1e-6);
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
}
