//! GraphRAG retrieval extension for framework.

use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use crate::hyperdim::{HVec10240, Hypervector};
use crate::retrieval::{GraphRagConfig, GraphRagResult, graph_rag_retrieve_generic};
use tracing::instrument;

impl<H: Hypervector> ChaoticSemanticFramework<H> {
    /// GraphRAG retrieval: similarity + graph traversal hybrid.
    ///
    /// Combines vector similarity with graph traversal for unified retrieval:
    /// 1. Anchor: probe(query, anchor_top_k) → seed set
    /// 2. Expand: traverse from each anchor
    /// 3. Score: similarity_weight * cosine + graph_weight * (1/(1+hops)) * strength
    /// 4. Dedup + rank by score.
    #[instrument(err, skip(self, query, config))]
    // Lock needed for concept and association access
    #[allow(clippy::significant_drop_tightening)]
    pub async fn probe_with_graph(
        &self,
        query: H,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        self.validate_top_k(config.anchor_top_k)?;
        self.validate_top_k(config.final_top_k)?;

        let (concepts, associations) = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            (sing.all_concepts(&ns), sing.all_associations(&ns))
        };

        graph_rag_retrieve_generic(&query, &concepts, &associations, &config)
    }

    /// GraphRAG retrieval using configured embedding provider for text query.
    #[instrument(err, skip(self, text, config))]
    pub async fn probe_text_with_graph(
        &self,
        text: &str,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        let embedding = self.embedding_provider.embed(text).await?;
        let query_hv10240: HVec10240 = self
            .embedding_provider
            .project(&embedding, &self.projection);
        // Convert HVec10240 to H using Hypervector trait methods
        let query_bytes = query_hv10240.to_bytes();
        let query: H =
            H::from_bytes(&query_bytes).map_err(|e| crate::error::MemoryError::InvalidInput {
                field: "query".to_string(),
                reason: format!("failed to convert hypervector: {e}"),
            })?;
        self.probe_with_graph(query, config).await
    }
}
