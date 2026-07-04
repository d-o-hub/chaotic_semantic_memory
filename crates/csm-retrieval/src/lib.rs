//! Retrieval backends for chaotic_semantic_memory.
//!
//! This crate provides:
//! - BM25 keyword search
//! - Hybrid retrieval (combining semantic and keyword)
//! - GraphRAG retrieval
//! - Reranking with MMR and recency decay

mod bm25;
mod graph_rag;
mod hybrid;
mod rerank;

pub use bm25::{Bm25Config, Bm25Index};
pub use graph_rag::{GraphRagConfig, GraphRagResult, graph_rag_retrieve};
pub use hybrid::{
    HybridConfig, HybridMode, HybridResult, RetrievalAbstention, merge_results,
    merge_results_checked, normalize_scores,
};
pub use rerank::{MmrReranker, RecencyDecayReranker, RerankCandidate, Reranker, parse_rerankers};

#[cfg(feature = "rerank-cross")]
pub use rerank::CrossEncoderReranker;
