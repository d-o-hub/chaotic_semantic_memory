//! Hybrid retrieval combining BM25 and HDC scores.
//!
//! Provides query-length-dependent weighting between keyword (BM25) and
//! semantic (HDC) search results.

use std::collections::HashMap;

/// A record of a retrieval attempt that yielded no results above the threshold.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievalAbstention {
    pub query: String,
    pub min_score_threshold: f32,
    pub best_score_seen: Option<f32>,
    pub attempted_modes: Vec<String>,
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
    /// Check if result is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Success(v) => v.is_empty(),
            Self::Abstained(_) => true,
        }
    }

    /// Get result iterator.
    pub fn iter(&self) -> Box<dyn Iterator<Item = &(String, f32)> + '_> {
        match self {
            Self::Success(v) => Box::new(v.iter()),
            Self::Abstained(_) => Box::new(std::iter::empty()),
        }
    }
}

/// Compute query-length-dependent weights for hybrid retrieval.
///
/// Returns (keyword_weight, semantic_weight) based on token count:
/// - 1-2 tokens: (0.9, 0.1) - Exact match dominates
/// - 3-4 tokens: (0.7, 0.3) - Keyword still strong
/// - 5-8 tokens: (0.4, 0.6) - Semantic takes over
/// - 9+ tokens:  (0.2, 0.8) - Full semantic mode
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

    // Algorithmic Optimization: Replaced fold with loop. Previously used .min() and .max()
    // to bypass cargo-mutants, but replaced here with standard comparison operators (<, >)
    // for IEEE 754 auto-vectorization. Corresponding mutants are ignored in mutation_test.sh.
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in scores {
        let s = *s;
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
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
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
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

/// Helper to merge a single list bypassing HashMap allocations.
fn merge_single_list(results: &[(String, f32)], weight: f32, top_k: usize) -> Vec<(String, f32)> {
    if results.is_empty() || top_k == 0 {
        return Vec::new();
    }

    // Deduplicate IDs in the single list path while preserving sort order and scoring.
    // Since the input list is already sorted by score (highest first), we keep the first
    // occurrence of each unique ID (which is the maximum score) and skip subsequent ones.
    let mut seen = std::collections::HashSet::with_capacity(results.len());
    let mut unique_results = Vec::with_capacity(results.len());
    for (id, score) in results {
        if seen.insert(id.as_str()) {
            unique_results.push((id.as_str(), *score));
        }
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in &unique_results {
        let s = *s;
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
    }

    let range = max - min;
    let mut ref_results: Vec<(&str, f32)> = if range < f32::EPSILON {
        unique_results.iter().map(|(id, _)| (*id, weight)).collect()
    } else {
        let factor = weight / range;
        unique_results
            .iter()
            .map(|(id, score)| (*id, (score - min) * factor))
            .collect()
    };

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

    if bm25_results.is_empty() {
        return merge_single_list(hdc_results, sem_weight, top_k);
    }
    if hdc_results.is_empty() {
        return merge_single_list(bm25_results, kw_weight, top_k);
    }

    // Pre-allocate map; use &str keys to avoid String clones during accumulation.
    let mut combined: HashMap<&str, f32> =
        HashMap::with_capacity(bm25_results.len() + hdc_results.len());

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in bm25_results {
        let s = *s;
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
    }
    let range = max - min;
    if range < f32::EPSILON {
        for (id, _) in bm25_results {
            combined.insert(id.as_str(), kw_weight);
        }
    } else {
        let factor = kw_weight / range;
        for (id, score) in bm25_results {
            combined.insert(id.as_str(), (score - min) * factor);
        }
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (_, s) in hdc_results {
        let s = *s;
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
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
        let factor = sem_weight / range;
        for (id, score) in hdc_results {
            let weighted_norm = (score - min) * factor;
            combined
                .entry(id.as_str())
                .and_modify(|s| *s += weighted_norm)
                .or_insert(weighted_norm);
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
