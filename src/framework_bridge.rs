//! Framework extensions for Semantic Bridge operations.

use tracing::instrument;
use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use crate::semantic_bridge::{BridgeHit, MemoryPacket, SemanticReranker};

impl ChaoticSemanticFramework {
    /// Natural language query via the Semantic Bridge.
    #[instrument(skip(self, query, reranker), fields(query = %query))]
    pub async fn bridge_query(
        &self,
        query: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<Vec<BridgeHit>> {
        let bridge = self.bridge_retrieval().await?;
        let sing = self.singularity.read().await;
        bridge.query(&self.namespace, &sing, query, top_k, reranker)
    }

    /// Natural language memory packet retrieval.
    #[instrument(skip(self, query, reranker), fields(query = %query))]
    pub async fn bridge_packet(
        &self,
        query: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<MemoryPacket> {
        let bridge = self.bridge_retrieval().await?;
        let sing = self.singularity.read().await;
        bridge.memory_packet(&self.namespace, &sing, query, top_k, reranker)
    }

    async fn bridge_retrieval(&self) -> Result<crate::bridge_retrieval::BridgeRetrieval> {
        use crate::bridge_retrieval::BridgeRetrieval;
        use crate::encoder::TextEncoder;
        use crate::semantic_bridge::{BridgeConfig, ConceptGraph};

        let encoder = TextEncoder::new();
        let mut graph = ConceptGraph::new();

        if let Some(ref persistence) = self.persistence {
            let concepts = persistence.load_all_canonical_concepts().await?;
            for c in concepts {
                graph.add_concept(c);
            }
        }

        Ok(BridgeRetrieval::new(encoder, graph, BridgeConfig::default()))
    }
}
