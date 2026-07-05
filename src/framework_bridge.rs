//! Framework wrappers for bridge retrieval.
//!
//! Provides async wrappers for bridge retrieval operations, integrating
//! with the ChaoticSemanticFramework's singularity lock management.

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::bridge_persistence::persist_absence;
use crate::bridge_retrieval::BridgeRetrieval;
use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::hybrid::{HybridResult, RetrievalAbstention};
use crate::semantic_bridge::{MemoryPacket, SemanticReranker};
use csm_core::error::Result;

impl ChaoticSemanticFramework {
    /// Execute bridge retrieval query with semantic expansion.
    ///
    /// Acquires singularity read lock and delegates to `BridgeRetrieval::query`.
    pub async fn probe_bridge_text(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
    ) -> Result<HybridResult> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        let ns = self.namespace.read().await;
        let (hits, best_score) =
            bridge.query_with_best_score(&ns, &singularity, query, top_k, None)?;

        if hits.is_empty() {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: bridge.config().deterministic_weight, // Approximate
                best_score_seen: best_score,
                attempted_modes: vec!["Bridge".to_string()],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if let Some(ref store) = self.persistence {
                let _ = persist_absence(&abstention, store.as_ref()).await;
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            let results = hits
                .into_iter()
                .map(|h| (h.id, h.scores.final_score))
                .collect();
            Ok(HybridResult::Success(results))
        }
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
    ) -> Result<HybridResult> {
        self.validate_top_k(top_k)?;
        let singularity = self.singularity.read().await;
        let ns = self.namespace.read().await;
        let (hits, best_score) =
            bridge.query_with_best_score(&ns, &singularity, query, top_k, Some(reranker))?;

        if hits.is_empty() {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: bridge.config().deterministic_weight,
                best_score_seen: best_score,
                attempted_modes: vec!["BridgeRerank".to_string()],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if let Some(ref store) = self.persistence {
                let _ = persist_absence(&abstention, store.as_ref()).await;
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            let results = hits
                .into_iter()
                .map(|h| (h.id, h.scores.final_score))
                .collect();
            Ok(HybridResult::Success(results))
        }
    }

    /// Execute bridge retrieval query with metadata filtering.
    ///
    /// Pre-filters concepts by metadata before bridge retrieval.
    // Singularity lock needed for filtered retrieval
    #[allow(clippy::significant_drop_tightening)]
    pub async fn probe_bridge_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        bridge: &BridgeRetrieval,
        filter: &MetadataFilter,
    ) -> Result<HybridResult> {
        self.validate_top_k(top_k)?;
        Self::validate_metadata_filter(filter)?;
        let singularity = self.singularity.read().await;
        let ns = self.namespace.read().await;

        // Get filtered concept IDs first
        let query_hv = bridge.encoder().encode(query);
        let filtered_results = singularity.find_similar_filtered(&ns, &query_hv, top_k, filter);
        let filtered_ids: std::collections::HashSet<String> = filtered_results
            .as_ref()
            .iter()
            .map(|(id, _)| id.clone())
            .collect();

        // Run full bridge query and filter results
        let (hits, best_score) = bridge.query_with_best_score(&ns, &singularity, query, top_k, None)?;
        drop(singularity);
        let filtered_hits: Vec<(String, f32)> = hits
            .into_iter()
            .filter(|hit| filtered_ids.contains(&hit.id))
            .map(|hit| (hit.id, hit.scores.final_score))
            .collect();

        if filtered_hits.is_empty() {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: bridge.config().deterministic_weight,
                best_score_seen: best_score,
                attempted_modes: vec!["BridgeFiltered".to_string()],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if let Some(ref store) = self.persistence {
                let _ = persist_absence(&abstention, store.as_ref()).await;
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            Ok(HybridResult::Success(filtered_hits))
        }
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
        let ns = self.namespace.read().await;
        bridge.memory_packet(&ns, &singularity, query, top_k, None)
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
        let ns = self.namespace.read().await;
        bridge.memory_packet(&ns, &singularity, query, top_k, Some(reranker))
    }
}

#[cfg(test)]
mod tests {
    // Exact float comparisons for confidence test assertions

    use crate::framework_builder::FrameworkBuilder;
    use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
    use crate::singularity::ConceptBuilder;
    use csm_core::encoder::TextEncoder;

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
        assert!((packet.confidence).abs() < f32::EPSILON);
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
        assert!(results.iter().any(|(id, _)| id == "test-concept"));
    }
}
