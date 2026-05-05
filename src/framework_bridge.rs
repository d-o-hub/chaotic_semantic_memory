//! Framework wrappers for bridge retrieval.
//!
//! Provides async wrappers for bridge retrieval operations, integrating
//! with the ChaoticSemanticFramework's singularity lock management.

use crate::bridge_retrieval::BridgeRetrieval;
use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use crate::semantic_bridge::{BridgeHit, MemoryPacket, SemanticReranker};

impl ChaoticSemanticFramework {
    /// Execute bridge retrieval query with semantic expansion.
    ///
    /// Acquires singularity read lock and delegates to `BridgeRetrieval::query`.
    pub async fn probe_bridge_text(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
    ) -> Result<Vec<BridgeHit>> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        bridge.query(&self.namespace, &singularity, query, top_k, None)
    }

    /// Execute bridge retrieval query with optional reranker.
    ///
    /// Acquires singularity read lock and delegates to `BridgeRetrieval::query`
    /// with the provided semantic reranker.
    pub async fn probe_bridge_text_with_reranker(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
        reranker: &dyn SemanticReranker,
    ) -> Result<Vec<BridgeHit>> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        bridge.query(&self.namespace, &singularity, query, top_k, Some(reranker))
    }

    /// Execute bridge retrieval query with metadata filtering.
    ///
    /// Pre-filters concepts by metadata before bridge retrieval.
    #[allow(clippy::significant_drop_tightening)] // Singularity lock needed for filtered retrieval
    pub async fn probe_bridge_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
        filter: &MetadataFilter,
    ) -> Result<Vec<BridgeHit>> {
        self.validate_top_k(top_k)?;
        Self::validate_metadata_filter(filter)?;
        let singularity = self.singularity.read().await;

        // Get filtered concept IDs first
        let query_hv = bridge.encoder().encode(query);
        let filtered_results =
            singularity.find_similar_filtered(&self.namespace, &query_hv, top_k, filter);
        let filtered_ids: std::collections::HashSet<String> = filtered_results
            .as_ref()
            .iter()
            .map(|(id, _)| id.clone())
            .collect();

        // Run full bridge query and filter results
        let hits = bridge.query(&self.namespace, &singularity, query, top_k, None)?;
        let filtered_hits: Vec<BridgeHit> = hits
            .into_iter()
            .filter(|hit| filtered_ids.contains(&hit.id))
            .collect();

        Ok(filtered_hits)
    }

    /// Compile memory packet from bridge retrieval results.
    ///
    /// Acquires singularity read lock and delegates to
    /// `BridgeRetrieval::memory_packet`.
    pub async fn memory_packet_text(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
    ) -> Result<MemoryPacket> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        bridge.memory_packet(&self.namespace, &singularity, query, top_k, None)
    }

    /// Compile memory packet with optional reranker.
    ///
    /// Acquires singularity read lock and delegates to
    /// `BridgeRetrieval::memory_packet` with the provided reranker.
    pub async fn memory_packet_text_with_reranker(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
        reranker: &dyn SemanticReranker,
    ) -> Result<MemoryPacket> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        bridge.memory_packet(&self.namespace, &singularity, query, top_k, Some(reranker))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparisons for confidence test assertions

    use crate::encoder::TextEncoder;
    use crate::framework_builder::FrameworkBuilder;
    use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
    use crate::singularity::ConceptBuilder;

    #[tokio::test]
    async fn test_probe_bridge_text_empty() {
        let framework = FrameworkBuilder::new().build().await.unwrap();
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = crate::bridge_retrieval::BridgeRetrieval::with_defaults(encoder, graph);

        let results = framework
            .probe_bridge_text("test query", 10, &bridge)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_memory_packet_text_empty() {
        let framework = FrameworkBuilder::new().build().await.unwrap();
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = crate::bridge_retrieval::BridgeRetrieval::with_defaults(encoder, graph);

        let packet = framework
            .memory_packet_text("test query", 10, &bridge)
            .await
            .unwrap();
        assert!(packet.facts.is_empty());
        assert_eq!(packet.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_probe_bridge_text_with_concepts() {
        let framework = FrameworkBuilder::new().build().await.unwrap();
        let encoder = TextEncoder::new();

        // Add concept to framework
        let concept = ConceptBuilder::new("test-concept")
            .with_vector(encoder.encode("agent memory system"))
            .build()
            .unwrap();
        framework
            .inject_concept(concept.id.clone(), concept.vector)
            .await
            .unwrap();

        // Create bridge with matching canonical concept
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("agent-memory")
                .with_label("ai-memory"),
        );

        let bridge = crate::bridge_retrieval::BridgeRetrieval::with_defaults(encoder, graph);

        let results = framework
            .probe_bridge_text("agent memory", 10, &bridge)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().any(|h| h.id == "test-concept"));
    }
}
