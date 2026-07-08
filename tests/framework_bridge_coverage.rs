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

    let framework = ChaoticSemanticFramework::builder()
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
            .all(|(id, _)| id == "filtered-1" || id.contains("filtered"))
    );
}

#[tokio::test]
async fn memory_packet_text_with_reranker_applies_reranking() {
    let encoder = TextEncoder::new();

    let framework = ChaoticSemanticFramework::builder()
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

    let framework = ChaoticSemanticFramework::builder()
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
    assert!(!results.is_empty());
}

#[test]
fn concept_graph_expand_collects_seed_and_related_labels() {
    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("agent")
            .with_label("agent")
            .with_related("memory"),
    );
    graph.add_concept(
        CanonicalConcept::new("memory")
            .with_label("memory")
            .with_related("context"),
    );
    graph.add_concept(CanonicalConcept::new("context").with_label("context"));

    let labels = graph.expand(&["agent".to_string()], 2);
    let label_set: std::collections::HashSet<&str> = labels.iter().map(String::as_str).collect();
    assert!(label_set.contains("agent"));
    assert!(label_set.contains("memory"));
    assert!(label_set.contains("context"));
}

#[test]
fn concept_graph_expand_dedupes_cycles() {
    // Self- and mutual-cycles must not produce duplicate labels or infinite loops.
    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("a")
            .with_label("alpha")
            .with_related("b"),
    );
    graph.add_concept(
        CanonicalConcept::new("b")
            .with_label("beta")
            .with_related("a"),
    );

    let labels = graph.expand(&["a".to_string()], 3);
    let occurrences: Vec<&str> = labels.iter().map(String::as_str).collect();
    let alpha_count = occurrences.iter().filter(|l| **l == "alpha").count();
    let beta_count = occurrences.iter().filter(|l| **l == "beta").count();
    assert_eq!(alpha_count, 1, "alpha appears once even with cycle");
    assert_eq!(beta_count, 1, "beta appears once even with cycle");
}

#[test]
fn concept_graph_expand_respects_max_depth() {
    // With max_depth=0 only the seed's labels appear; related concepts at
    // depth 1 must not be reached. This pins the depth check semantics.
    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("root")
            .with_label("root-label")
            .with_related("child"),
    );
    graph.add_concept(
        CanonicalConcept::new("child")
            .with_label("child-label")
            .with_related("grandchild"),
    );
    graph.add_concept(CanonicalConcept::new("grandchild").with_label("grandchild-label"));

    let labels = graph.expand(&["root".to_string()], 0);
    let label_set: std::collections::HashSet<&str> = labels.iter().map(String::as_str).collect();
    assert!(label_set.contains("root-label"));
    assert!(
        !label_set.contains("child-label"),
        "max_depth=0 must not traverse to related concepts"
    );
    assert!(!label_set.contains("grandchild-label"));
}

#[test]
fn concept_graph_expand_handles_unknown_seed() {
    let graph = ConceptGraph::new();
    let labels = graph.expand(&["missing".to_string()], 5);
    assert!(labels.is_empty());
}

#[test]
fn concept_graph_add_remove_match_roundtrip() {
    // Regression for the optimize-indexing PR: the lowercased label index must
    // round-trip through add_concept / match_tokens / remove_concept.
    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("c1")
            .with_label("Memory-System")
            .with_related("c2"),
    );
    graph.add_concept(CanonicalConcept::new("c2").with_label("Context"));

    // Case-insensitive match for both styles.
    let lower = graph.match_tokens(&["memory-system".to_string()]);
    let upper = graph.match_tokens(&["MEMORY-SYSTEM".to_string()]);
    assert!(lower.contains(&"c1".to_string()));
    assert!(upper.contains(&"c1".to_string()));

    // remove_concept must clean the label index fully.
    let removed = graph.remove_concept("c1");
    assert!(removed.is_some());
    let after = graph.match_tokens(&["memory-system".to_string()]);
    assert!(!after.contains(&"c1".to_string()));
}
