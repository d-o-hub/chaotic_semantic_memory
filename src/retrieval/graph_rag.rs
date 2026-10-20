//! Root facade for the GraphRAG implementation owned by `csm-retrieval`.

pub use csm_retrieval::{
    GraphRagConfig, GraphRagResult, MAX_GRAPH_RAG_ANCHOR_TOP_K, MAX_GRAPH_RAG_FINAL_TOP_K,
    MAX_GRAPH_RAG_HOPS, graph_rag_retrieve,
};
