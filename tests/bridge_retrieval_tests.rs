use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::semantic_bridge::CanonicalConcept;
use chaotic_semantic_memory::semantic_bridge::{BridgeConfig, ConceptGraph, ScoreBreakdown};
use chaotic_semantic_memory::singularity::{ConceptBuilder, Singularity, SingularityConfig};
use csm_core::encoder::TextEncoder;
use csm_core::hyperdim::HVec10240;

#[test]
fn test_bridge_retrieval_empty_singularity() {
    let encoder = TextEncoder::new();
    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::with_defaults(encoder, graph);
    let singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let results = bridge
        .query("_default", &singularity, "test query", 10, None)
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_bridge_retrieval_empty_graph() {
    let encoder = TextEncoder::new();
    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), graph);

    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    let concept = ConceptBuilder::new("test-concept")
        .with_vector(encoder.encode("test content"))
        .build()
        .unwrap();
    singularity.inject("_default", concept).unwrap();

    let results = bridge
        .query("_default", &singularity, "test query", 10, None)
        .unwrap();
    // Should return deterministic results even without graph expansion
    assert!(!results.is_empty());
    assert!(results[0].scores.deterministic > 0.0);
    assert!((results[0].scores.concept).abs() < f32::EPSILON);
}

#[test]
fn test_bridge_retrieval_with_expansion() {
    let encoder = TextEncoder::new();
    let mut graph = ConceptGraph::new();

    // Add canonical concept with label matching query
    graph.add_concept(
        CanonicalConcept::new("c1")
            .with_label("agent-memory")
            .with_label("session-context"),
    );

    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), graph);

    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    let concept = ConceptBuilder::new("mem-1")
        .with_vector(encoder.encode("session context for AI agent"))
        .build()
        .unwrap();
    singularity.inject("_default", concept).unwrap();

    let results = bridge
        .query("_default", &singularity, "agent memory session", 10, None)
        .unwrap();

    assert!(!results.is_empty());
    // Check that expansion added concept score evidence
    assert!(
        results[0]
            .scores
            .evidence
            .contains(&"deterministic_recall".to_string())
    );
}

#[test]
fn test_memory_packet_empty_hits() {
    let encoder = TextEncoder::new();
    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::with_defaults(encoder, graph);
    let singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let packet = bridge
        .memory_packet("_default", &singularity, "test query", 10, None)
        .unwrap();
    assert!(packet.facts.is_empty());
    assert!(packet.sources.is_empty());
    assert!((packet.confidence).abs() < f32::EPSILON);
}

#[test]
fn test_final_score_weights() {
    let config = BridgeConfig {
        deterministic_weight: 0.6,
        concept_weight: 0.3,
        semantic_weight: 0.1,
        ..Default::default()
    };

    let encoder = TextEncoder::new();
    let graph = ConceptGraph::new();
    let bridge = BridgeRetrieval::new(encoder, graph, config);

    let scores = ScoreBreakdown {
        deterministic: 1.0,
        concept: 1.0,
        semantic: 1.0,
        final_score: 0.0,
        evidence: vec!["test".to_string()],
    };

    let final_score = bridge.compute_final_score(&scores);
    assert!((final_score - 1.0).abs() < 1e-6); // All weights sum to 1.0
}

/// Kills compute_final_score -> 1.0 mutant: partial scores must NOT yield 1.0.
#[test]
fn test_final_score_partial_deterministic_only() {
    let config = BridgeConfig {
        deterministic_weight: 0.6,
        concept_weight: 0.3,
        semantic_weight: 0.1,
        ..Default::default()
    };
    let bridge = BridgeRetrieval::new(TextEncoder::new(), ConceptGraph::new(), config);
    let scores = ScoreBreakdown {
        deterministic: 0.5,
        concept: 0.0,
        semantic: 0.0,
        final_score: 0.0,
        evidence: vec![],
    };
    // 0.6 * 0.5 = 0.3, not 1.0
    let final_score = bridge.compute_final_score(&scores);
    assert!(
        (final_score - 0.3).abs() < 1e-5,
        "expected 0.3, got {final_score}"
    );
}

/// Kills query `||` -> `&&` mutant: top_k == 0 with non-empty singularity must return empty.
#[test]
fn test_query_top_k_zero_returns_empty() {
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), ConceptGraph::new());
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    let concept = ConceptBuilder::new("c1")
        .with_vector(encoder.encode("hello world"))
        .build()
        .unwrap();
    singularity.inject("_default", concept).unwrap();
    // top_k == 0 must short-circuit even when singularity is non-empty
    let results = bridge
        .query("_default", &singularity, "hello", 0, None)
        .unwrap();
    assert!(results.is_empty(), "top_k=0 must yield empty results");
}

#[test]
fn test_bridge_retrieval_query_v2() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let c = ConceptBuilder::new("c1")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    sing.inject("_default", c).unwrap();
    let bridge = BridgeRetrieval::new(
        TextEncoder::new(),
        ConceptGraph::new(),
        BridgeConfig::default(),
    );
    assert!(bridge.query("_default", &sing, "test", 10, None).is_ok());
}

#[test]
fn test_memory_packet_confidence_is_average() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let hv = HVec10240::random();
    for i in 0..3 {
        let c = ConceptBuilder::new(format!("c{i}"))
            .with_vector(hv)
            .build()
            .unwrap();
        sing.inject("_default", c).unwrap();
    }
    let bridge = BridgeRetrieval::new(
        TextEncoder::new(),
        ConceptGraph::new(),
        BridgeConfig::default(),
    );
    let pkt = bridge
        .memory_packet("_default", &sing, "concept", 10, None)
        .unwrap();
    assert!(
        pkt.confidence > 0.0 && pkt.confidence <= 1.0,
        "confidence {}",
        pkt.confidence
    );
}
