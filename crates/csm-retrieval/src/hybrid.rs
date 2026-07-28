//! Hybrid retrieval combining BM25 and HDC scores.
//!
//! Provides query-length-dependent weighting between keyword (BM25) and
//! semantic (HDC) search results.

use std::collections::HashMap;

/// A record of a retrieval attempt that yielded no results above the threshold.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievalAbstention {
    /// The original query string
    pub query: String,
    /// The minimum score threshold that was required
    pub min_score_threshold: f32,
    /// The best score seen during this attempt
    pub best_score_seen: Option<f32>,
    /// The retrieval modes attempted (e.g., "Auto", "SemanticOnly")
    pub attempted_modes: Vec<String>,
    /// Timestamp when this abstention occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result of a hybrid retrieval operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HybridResult {
    /// Results found above threshold.
    Success(Vec<(String, f32)>),
    /// No results met the confidence threshold.
    Abstained(RetrievalAbstention),
}

impl HybridResult {
    /// Check if the result is a success and contains entries.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Success(v) => v.is_empty(),
            Self::Abstained(_) => true,
        }
    }

    /// Get an iterator over results if it is a success.
    pub fn iter(&self) -> Box<dyn Iterator<Item = &(String, f32)> + '_> {
        match self {
            Self::Success(v) => Box::new(v.iter()),
            Self::Abstained(_) => Box::new(std::iter::empty()),
        }
    }
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
///
/// # Note
/// For performance-critical pipelines (like search/retrieval hot paths), prefer
/// using [`normalize_scores_in_place`] to completely bypass vector allocations
/// and redundant string cloning.
pub fn normalize_scores(scores: &[(String, f32)]) -> Vec<(String, f32)> {
    if scores.is_empty() {
        return Vec::new();
    }

    // Algorithmic Optimization: Replaced fold with a simple loop. Uses .min() and .max()
    // to prevent cargo-mutants from generating equivalent mutants on relational operators.
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in scores {
        let s = *s;
        min = min.min(s);
        max = max.max(s);
    }

    let range = max - min;
    if range < f32::EPSILON {
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

/// Normalize scores in place to [0, 1] range using min-max normalization.
///
/// This is the preferred method in high-frequency/hot query pathways because
/// it mutates existing score vectors, avoiding heap allocations and string cloning.
pub fn normalize_scores_in_place(scores: &mut [(String, f32)]) {
    if scores.is_empty() {
        return;
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in &*scores {
        let s = *s;
        min = min.min(s);
        max = max.max(s);
    }

    let range = max - min;
    if range < f32::EPSILON {
        for (_, score) in scores {
            *score = 1.0;
        }
        return;
    }

    let inv_range = 1.0 / range;
    for (_, score) in scores {
        *score = (*score - min) * inv_range;
    }
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
    top_k: usize,
) -> Vec<(String, f32)> {
    if top_k == 0 {
        return Vec::new();
    }

    let (kw_weight, sem_weight) = weights;

    // Pre-allocate map; use &str keys to avoid String clones during accumulation.
    let mut combined: HashMap<&str, f32> =
        HashMap::with_capacity(bm25_results.len() + hdc_results.len());

    // Fold min/max then insert — no intermediate Vec allocation.
    if !bm25_results.is_empty() {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for (_, s) in bm25_results {
            let s = *s;
            min = min.min(s);
            max = max.max(s);
        }
        let range = max - min;
        if range < f32::EPSILON {
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
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for (_, s) in hdc_results {
            let s = *s;
            min = min.min(s);
            max = max.max(s);
        }
        let range = max - min;
        if range < f32::EPSILON {
            for (id, _) in hdc_results {
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

    // Perform top-k selection on references to delay string cloning/allocation.
    let mut ref_results: Vec<(&str, f32)> = combined.into_iter().collect();

    // O(N) top-k selection, then sort only the retained slice.
    if ref_results.len() > top_k {
        ref_results.select_nth_unstable_by(top_k, |a, b| b.1.total_cmp(&a.1));
        ref_results.truncate(top_k);
    }
    ref_results.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

    ref_results
        .into_iter()
        .map(|(id, score)| (id.to_string(), score))
        .collect()
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
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    // Exact float comparisons for weight test assertions

    use super::*;

    #[test]
    fn test_compute_weights() {
        let cases = vec![
            (1, 0.9, 0.1),
            (2, 0.9, 0.1),
            (3, 0.7, 0.3),
            (4, 0.7, 0.3),
            (5, 0.4, 0.6),
            (8, 0.4, 0.6),
            (9, 0.2, 0.8),
            (100, 0.2, 0.8),
        ];
        for (tc, expected_kw, expected_sem) in cases {
            let (kw, sem) = compute_weights(tc);
            assert!(
                (kw - expected_kw).abs() < 1e-6,
                "kw weight assertion failed for query token count {tc}"
            );
            assert!(
                (sem - expected_sem).abs() < 1e-6,
                "sem weight assertion failed for query token count {tc}"
            );
        }
    }

    #[test]
    fn test_normalize_scores() {
        let scores = vec![
            ("a".to_string(), 10.0),
            ("b".to_string(), 15.0),
            ("c".to_string(), 20.0),
        ];
        let normalized = normalize_scores(&scores);
        assert!((normalized[0].1 - 0.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.5).abs() < 1e-6);
        assert!((normalized[2].1 - 1.0).abs() < 1e-6);

        let scores = vec![("a".to_string(), 2.0), ("b".to_string(), 0.0)];
        let normalized = normalize_scores(&scores);
        assert!((normalized[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized[1].1 - 0.0).abs() < 1e-6);

        assert!(normalize_scores(&[]).is_empty());

        let scores = vec![("a".to_string(), 5.0), ("b".to_string(), 5.0)];
        let normalized = normalize_scores(&scores);
        assert!((normalized[0].1 - 1.0).abs() < 1e-6);
        assert!((normalized[1].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_scores_in_place_parity() {
        let mut empty = Vec::new();
        normalize_scores_in_place(&mut empty);
        assert!(empty.is_empty());
        let mut single = vec![("a".to_string(), 10.0)];
        normalize_scores_in_place(&mut single);
        assert!((single[0].1 - 1.0).abs() < 1e-6);
        let mut multi = vec![
            ("a".to_string(), 10.0),
            ("b".to_string(), 15.0),
            ("c".to_string(), 20.0),
        ];
        let original = multi.clone();
        normalize_scores_in_place(&mut multi);
        let expected = normalize_scores(&original);
        assert_eq!(multi.len(), expected.len());
        for i in 0..multi.len() {
            assert_eq!(multi[i].0, expected[i].0);
            assert!((multi[i].1 - expected[i].1).abs() < 1e-6);
        }
    }

    #[test]
    fn test_merge_results() {
        let bm25 = vec![("doc1".to_string(), 1.0), ("doc2".to_string(), 0.5)];
        let hdc = vec![("doc1".to_string(), 0.5), ("doc3".to_string(), 1.0)];
        let merged = merge_results(&bm25, &hdc, (0.5, 0.5), 10);
        assert!(merged.iter().any(|(id, _)| id == "doc1"));
        assert!(merged.iter().any(|(id, _)| id == "doc2"));
        assert!(merged.iter().any(|(id, _)| id == "doc3"));

        let bm25 = vec![("doc1".to_string(), 1.0)];
        let hdc = vec![("doc1".to_string(), 1.0)];
        let merged = merge_results(&bm25, &hdc, (0.9, 0.1), 10);
        assert!(merged.iter().any(|(id, s)| id == "doc1" && *s > 0.0));

        assert!(merge_results(&[], &[], (0.5, 0.5), 10).is_empty());
        assert_eq!(
            merge_results(&[("a".to_string(), 1.0)], &[], (0.5, 0.5), 10).len(),
            1
        );
        assert_eq!(
            merge_results(&[], &[("a".to_string(), 1.0)], (0.5, 0.5), 10).len(),
            1
        );
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

        let merged = merge_results(&bm25, &hdc, weights, 10);

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
        let merged = merge_results(&bm25, &hdc, weights, 10);
        // Each gets kw_weight (0.5) + sem_weight (0.5) = 1.0
        assert_eq!(merged.len(), 2);
        for (_, score) in merged {
            assert!((score - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_range_epsilon_boundary() {
        let epsilon = f32::EPSILON;
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
        let epsilon = f32::EPSILON;
        let weights = (0.5, 0.5);

        // Case 1: range exactly epsilon (should normalize)
        let bm25 = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
        let hdc = vec![("d1".to_string(), epsilon), ("d2".to_string(), 0.0)];
        let merged = merge_results(&bm25, &hdc, weights, 10);
        let d1_score = merged.iter().find(|(id, _)| id == "d1").unwrap().1;
        let d2_score = merged.iter().find(|(id, _)| id == "d2").unwrap().1;
        assert!((d1_score - 1.0).abs() < 1e-6);
        assert!((d2_score - 0.0).abs() < 1e-6);

        // Case 2: range just below epsilon (should fallback to 1.0)
        let just_below = epsilon * 0.9;
        let bm25_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
        let hdc_small = vec![("d1".to_string(), just_below), ("d2".to_string(), 0.0)];
        let merged_small = merge_results(&bm25_small, &hdc_small, weights, 10);
        for (_, score) in merged_small {
            assert!((score - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_merge_results_top_k() {
        let bm25 = vec![
            ("d1".to_string(), 10.0),
            ("d2".to_string(), 8.0),
            ("d3".to_string(), 6.0),
        ];
        let hdc = vec![
            ("d1".to_string(), 10.0),
            ("d4".to_string(), 4.0),
            ("d5".to_string(), 2.0),
        ];
        let weights = (0.5, 0.5);

        // top_k = 2 should return exactly the 2 best elements: d1 and d2
        let merged = merge_results(&bm25, &hdc, weights, 2);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].0, "d1");
        assert_eq!(merged[1].0, "d2");

        // top_k = 1 should return only d1
        let merged_one = merge_results(&bm25, &hdc, weights, 1);
        assert_eq!(merged_one.len(), 1);
        assert_eq!(merged_one[0].0, "d1");

        // top_k = 0 should return empty
        let merged_zero = merge_results(&bm25, &hdc, weights, 0);
        assert!(merged_zero.is_empty());
    }

    #[test]
    fn test_merge_results_top_k_exact_boundary() {
        // When unique result count equals top_k, the partial-sort branch must NOT run.
        // Using `>=` instead of `>` would call select_nth_unstable_by(top_k) with
        // index == len and panic.
        let bm25 = vec![("d1".to_string(), 10.0), ("d2".to_string(), 8.0)];
        let hdc = vec![("d1".to_string(), 10.0), ("d2".to_string(), 8.0)];
        let weights = (0.5, 0.5);
        let merged = merge_results(&bm25, &hdc, weights, 2);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].0, "d1");
        assert_eq!(merged[1].0, "d2");
    }
}
