//! Hybrid retrieval combining BM25 and HDC scores.

pub use csm_retrieval::{
    HybridConfig, HybridMode, HybridResult, RetrievalAbstention, compute_weights, merge_results,
    normalize_scores, normalize_scores_in_place,
};
