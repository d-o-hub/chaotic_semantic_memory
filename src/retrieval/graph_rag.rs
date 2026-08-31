//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Re-exports GraphRAG implementation from `csm_retrieval` to satisfy single source
//! of truth and the 500 LOC per file limit.

pub use csm_retrieval::{GraphRagConfig, GraphRagResult, graph_rag_retrieve};
