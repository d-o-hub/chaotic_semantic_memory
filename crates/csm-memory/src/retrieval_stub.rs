pub mod index {
    use csm_core::Result;
    pub trait AnnIndex: Send + Sync + std::fmt::Debug {}
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub enum IndexBackend {
        BruteForce,
        Hnsw,
        Lsh,
    }
    pub fn create_index(_backend: &IndexBackend) -> Box<dyn AnnIndex> {
        todo!()
    }
    pub struct IndexStats;
}

pub mod bridge_retrieval {
    pub struct BridgeRetrieval;
}

pub mod graph_traversal {
    pub struct TraversalConfig;
}

pub mod retrieval {
    pub struct GraphRagConfig;
    pub struct GraphRagResult;
    pub async fn graph_rag_search() -> GraphRagResult { todo!() }
    pub mod rerank {
        pub struct RerankCandidate;
        pub trait Reranker {}
    }
}
