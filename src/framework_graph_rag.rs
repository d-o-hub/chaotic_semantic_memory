//! GraphRAG retrieval extension for framework.

use crate::framework::ChaoticSemanticFramework;
use crate::retrieval::{GraphRagConfig, GraphRagResult, graph_rag_retrieve};
use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;
use tracing::instrument;

impl ChaoticSemanticFramework {
    /// GraphRAG retrieval: similarity + graph traversal hybrid.
    ///
    /// Combines vector similarity with graph traversal for unified retrieval:
    /// 1. Anchor: probe(query, anchor_top_k) → seed set
    /// 2. Expand: traverse from each anchor
    /// 3. Score: similarity_weight * cosine + graph_weight * (1/(1+hops)) * strength
    /// 4. Dedupe + rank by score
    #[instrument(err, skip(self, query, config))]
    // Lock needed for concept and association access
    pub async fn probe_with_graph(
        &self,
        query: HVec10240,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        self.validate_top_k(config.anchor_top_k)?;
        self.validate_top_k(config.final_top_k)?;

        let (concepts, associations) = {
            let sing = self.singularity.read().await;
            let ns = self.namespace.read().await;
            (sing.all_concepts(&ns), sing.all_associations(&ns))
        };

        graph_rag_retrieve(&query, &concepts, &associations, &config)
    }

    /// GraphRAG retrieval using configured embedding provider for text query.
    #[instrument(err, skip(self, text, config))]
    pub async fn probe_text_with_graph(
        &self,
        text: &str,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>> {
        let embedding = self.embedding_provider.embed(text).await?;
        let query = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.probe_with_graph(query, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_with_graph_returns_relevant_results() {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        let vector = HVec10240::random();
        fw.inject_concept("graph".to_string(), vector)
            .await
            .unwrap();
        let config = GraphRagConfig {
            anchor_top_k: 5,
            max_hops: 2,
            min_assoc_strength: 0.1,
            similarity_weight: 0.7,
            graph_weight: 0.3,
            final_top_k: 5,
        };
        let results = fw.probe_with_graph(vector, config).await.unwrap();
        assert!(
            !results.is_empty(),
            "probe_with_graph must return at least one result"
        );
    }
}
