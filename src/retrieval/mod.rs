//! Retrieval modules for hybrid search.
//!
//! This crate supports hybrid retrieval combining:
//! - **BM25**: Keyword/exact matching for short queries
//! - **HDC**: Semantic similarity for natural language queries
//! - **GraphRAG**: Graph traversal + similarity for graph-heavy memory
//!
//! The hybrid approach uses query-length-dependent weighting:
//! - Short queries (1-2 tokens): 90% keyword, 10% semantic
//! - Long queries (9+ tokens): 20% keyword, 80% semantic

pub mod bm25;
pub mod graph_rag;
pub mod hybrid;

pub use bm25::{Bm25Config, Bm25Index};
pub use graph_rag::{GraphRagConfig, GraphRagResult, graph_rag_retrieve_generic};
pub use hybrid::{HybridConfig, HybridMode, compute_weights, merge_results, normalize_scores};
