//! Hybrid retrieval combining BM25 and HDC scores.
//!
//! **Implementation owner:** [`csm_retrieval`] (ADR-0094).
//! This module is a stable root façade; algorithms live in the owner crate.

pub use csm_retrieval::{
    HybridConfig, HybridMode, HybridResult, RetrievalAbstention, compute_weights, merge_results,
    normalize_scores,
};

#[cfg(test)]
mod facade_parity_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn compute_weights_matches_owner_contract() {
        assert_eq!(compute_weights(1), (0.9, 0.1));
        assert_eq!(compute_weights(3), (0.7, 0.3));
        assert_eq!(compute_weights(5), (0.4, 0.6));
        assert_eq!(compute_weights(9), (0.2, 0.8));
    }

    #[test]
    fn merge_results_includes_both_sources() {
        let bm25 = vec![("a".into(), 10.0), ("b".into(), 0.0)];
        let hdc = vec![("a".into(), 0.0), ("c".into(), 5.0)];
        let merged = merge_results(&bm25, &hdc, (0.5, 0.5));
        let ids: std::collections::HashSet<_> = merged.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains("a"));
        assert!(ids.contains("c"));
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn hybrid_result_empty_semantics() {
        assert!(HybridResult::Success(vec![]).is_empty());
        assert!(!HybridResult::Success(vec![("x".into(), 1.0)]).is_empty());
    }
}
