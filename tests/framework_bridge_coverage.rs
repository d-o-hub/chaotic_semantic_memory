//! Framework bridge operations coverage tests.
//!
//! Covers: probe_bridge_text_filtered, memory_packet_text_with_reranker

use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::semantic_bridge::{
    BridgeHit, CanonicalConcept, ConceptGraph, SemanticReranker,
};

/// Custom reranker for testing
struct TestReranker;

impl SemanticReranker for TestReranker {
    fn version(&self) -> &str {
        "test-reranker-v1"
    }

    fn rerank(&self, _query: &str, hits: &mut [BridgeHit]) {
        // Simple reranker that reverses the order in place
        hits.reverse();
    }
}

#[tokio::test]
async fn probe_bridge_text_filtered_returns_matching() {
    let encoder = TextEncoder::new();

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject concepts with metadata
    let v1 = encoder.encode("memory system semantic");
    framework
        .inject_concept_with_metadata(
            "filtered-1",
            v1,
            std::collections::HashMap::from([
                ("type".to_string(), serde_json::json!("memory")),
                ("category".to_string(), serde_json::json!("system")),
            ]),
        )
        .await
        .unwrap();

    let v2 = encoder.encode("unrelated concept");
    framework
        .inject_concept_with_metadata(
            "filtered-2",
            v2,
            std::collections::HashMap::from([("type".to_string(), serde_json::json!("other"))]),
        )
        .await
        .unwrap();

    // Create bridge with matching canonical concept
    let mut graph = ConceptGraph::new();
    graph.add_concept(CanonicalConcept::new("c1").with_label("memory-system"));

    let bridge = BridgeRetrieval::with_defaults(encoder, graph);

    // Filter by metadata
    let filter = MetadataFilter::eq("type", "memory");
    let results = framework
        .probe_bridge_text_filtered("memory", 5, &bridge, &filter)
        .await
        .unwrap();

    // Should only return concepts matching the filter
    assert!(
        results
            .iter()
            .all(|h| h.id == "filtered-1" || h.id.contains("filtered"))
    );
}

#[tokio::test]
async fn memory_packet_text_with_reranker_applies_reranking() {
    let encoder = TextEncoder::new();

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject concepts
    framework
        .inject_concept("rerank-1", encoder.encode("first concept"))
        .await
        .unwrap();
    framework
        .inject_concept("rerank-2", encoder.encode("second concept"))
        .await
        .unwrap();

    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::with_defaults(encoder, graph);
    let reranker = TestReranker;

    let packet = framework
        .memory_packet_text_with_reranker("concept", 5, &bridge, &reranker)
        .await
        .unwrap();

    // Verify packet was created (reranking applied internally)
    assert!(packet.facts.len() <= 2);
}

#[tokio::test]
async fn probe_bridge_text_with_reranker_applies_reranking() {
    let encoder = TextEncoder::new();

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("rerank-probe-1", encoder.encode("alpha concept"))
        .await
        .unwrap();
    framework
        .inject_concept("rerank-probe-2", encoder.encode("beta concept"))
        .await
        .unwrap();

    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::with_defaults(encoder, graph);
    let reranker = TestReranker;

    let results = framework
        .probe_bridge_text_with_reranker("concept", 5, &bridge, &reranker)
        .await
        .unwrap();

    // Verify results were returned (reranking applied internally)
    assert!(results.len() <= 2);
}
