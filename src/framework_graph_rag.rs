/// GraphRAG retrieval extension for framework.

use crate::error::Result;
use crate::hyperdim::Hypervector;
use crate::framework::ChaoticSemanticFramework;
use crate::retrieval::{GraphRagConfig, GraphRagResult};
use crate::retrieval::graph_rag::graph_rag_retrieve_generic;

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    /// Execute GraphRAG retrieval query.
    ///
    /// Combines vector similarity with graph expansion to discover
    /// non-obvious semantic associations.
    pub async fn probe_with_graph(
        &self,
        query: H,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        let (concepts, associations) = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            (sing.all_concepts(&ns), sing.all_associations(&ns))
        };

        graph_rag_retrieve_generic(&query, &concepts, &associations, &config)
    }

    /// Execute GraphRAG retrieval query using text input.
    pub async fn probe_text_with_graph(
        &self,
        text: &str,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        let embedding = self.embedding_provider.embed(text).await?;
        let query_f32 = self
            .embedding_provider
            .project(&embedding, &self.projection);
        let query = H::from_hvec(&query_f32);
        self.probe_with_graph(query, config).await
    }
}
