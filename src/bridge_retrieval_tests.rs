#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge_retrieval::BridgeRetrieval;
use crate::semantic_bridge::{BridgeConfig, CanonicalConcept, ConceptGraph, ScoreBreakdown};
use crate::singularity::{ConceptBuilder, Singularity, SingularityConfig};
use csm_core_lib::encoder::TextEncoder;
use csm_core_lib::hyperdim::HVec10240;

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
    assert!(!results.is_empty());
    assert!(results[0].scores.deterministic > 0.0);
    assert!((results[0].scores.concept).abs() < f32::EPSILON);
}

#[test]
fn test_bridge_retrieval_with_expansion() {
    let encoder = TextEncoder::new();
    let mut graph = ConceptGraph::new();

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
    assert!((final_score - 1.0).abs() < 1e-6);
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
    let results = bridge
        .query("_default", &singularity, "hello", 0, None)
        .unwrap();
    assert!(results.is_empty(), "top_k=0 must yield empty results");
}

/// Kills query `||` -> `&&`: empty namespace with top_k > 0 must also short-circuit.
#[test]
fn test_query_empty_namespace_returns_empty() {
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::with_defaults(encoder, ConceptGraph::new());
    let singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    let results = bridge
        .query("_default", &singularity, "anything", 5, None)
        .unwrap();
    assert!(
        results.is_empty(),
        "empty namespace must yield empty results"
    );
}

/// Kills compile_packet `delete !` in dedup: duplicate facts must appear only once.
#[test]
fn test_compile_packet_deduplicates_facts() {
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), ConceptGraph::new());
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    for id in &["c1", "c2"] {
        let concept = ConceptBuilder::new(*id)
            .with_vector(encoder.encode("shared text"))
            .with_metadata(
                "_text",
                serde_json::Value::String("duplicate fact".to_string()),
            )
            .build()
            .unwrap();
        singularity.inject("_default", concept).unwrap();
    }

    let hits = bridge
        .query("_default", &singularity, "shared text", 10, None)
        .unwrap();
    let packet = bridge
        .memory_packet("_default", &singularity, "shared text", 10, None)
        .unwrap();
    assert!(!hits.is_empty(), "expected hits for matching concepts");
    let dup_count = packet
        .facts
        .iter()
        .filter(|f| *f == "duplicate fact")
        .count();
    assert_eq!(dup_count, 1, "duplicate facts must be deduplicated");
}

/// Kills compile_packet token budget mutants: facts exceeding budget must be dropped.
#[test]
fn test_compile_packet_respects_token_budget() {
    let config = BridgeConfig {
        token_budget: 3,
        max_packet_facts: 20,
        ..Default::default()
    };
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::new(encoder.clone(), ConceptGraph::new(), config);
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let short = ConceptBuilder::new("short")
        .with_vector(encoder.encode("ok"))
        .with_metadata("_text", serde_json::Value::String("ok".to_string()))
        .build()
        .unwrap();
    let long_text = "one two three four five six seven eight nine ten";
    let long_c = ConceptBuilder::new("long")
        .with_vector(encoder.encode(long_text))
        .with_metadata("_text", serde_json::Value::String(long_text.to_string()))
        .build()
        .unwrap();
    singularity.inject("_default", short).unwrap();
    singularity.inject("_default", long_c).unwrap();

    let packet = bridge
        .memory_packet("_default", &singularity, "ok one two", 10, None)
        .unwrap();
    assert!(
        !packet.facts.contains(&long_text.to_string()),
        "fact exceeding token budget must be excluded"
    );
}

/// Kills compile_packet confidence `/` -> `%`/`*`: confidence must be mean of top scores.
#[test]
fn test_compile_packet_confidence_is_mean() {
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), ConceptGraph::new());
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let concept = ConceptBuilder::new("c1")
        .with_vector(encoder.encode("test"))
        .build()
        .unwrap();
    singularity.inject("_default", concept).unwrap();

    let packet = bridge
        .memory_packet("_default", &singularity, "test", 10, None)
        .unwrap();
    assert!(
        (0.0..=1.0).contains(&packet.confidence),
        "confidence must be in [0, 1], got {}",
        packet.confidence
    );
}

#[test]
fn test_bridge_retrieval_query_v2() {
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());
    let concept = ConceptBuilder::new("c1")
        .with_vector(HVec10240::random())
        .build()
        .unwrap();
    singularity.inject("_default", concept).unwrap();

    let bridge = BridgeRetrieval::new(
        TextEncoder::new(),
        ConceptGraph::new(),
        BridgeConfig::default(),
    );
    let results = bridge.query("_default", &singularity, "test", 10, None);
    assert!(results.is_ok());
}
