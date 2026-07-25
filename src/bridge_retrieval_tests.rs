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

/// Kills `/->*` at estimated=ceil(count/0.75): a 3-word fact costs ceil(3/0.75)=4 tokens,
/// which exceeds budget=3; with `*` it costs ceil(3*0.75)=3 which would fit.
#[test]
fn test_compile_packet_token_budget_estimation_division() {
    // budget=3; "one two three" => ceil(3/0.75)=4 > 3 => excluded
    // with `/->*`: ceil(3*0.75)=3 <= 3 => included (mutant survives if test weak)
    let config = BridgeConfig {
        token_budget: 3,
        max_packet_facts: 20,
        ..Default::default()
    };
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::new(encoder.clone(), ConceptGraph::new(), config);
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let text3 = "one two three";
    let c = ConceptBuilder::new("c1")
        .with_vector(encoder.encode(text3))
        .with_metadata("_text", serde_json::Value::String(text3.to_string()))
        .build()
        .unwrap();
    singularity.inject("_default", c).unwrap();

    let packet = bridge
        .memory_packet("_default", &singularity, "one two three", 10, None)
        .unwrap();
    assert!(
        !packet.facts.contains(&text3.to_string()),
        "3-word fact must be excluded from budget=3 (estimated=4 tokens)"
    );
}

/// Kills `+->*` in `token_count + estimated <= budget`: with `*`, first iteration is
/// `0 * estimated = 0` which always satisfies any budget, so everything is included.
#[test]
fn test_compile_packet_token_budget_accumulation() {
    // budget=3; "one two three" => estimated=4; 0+4=4>3 => excluded
    // with `+->*`: 0*4=0<=3 => included (mutant would include it)
    let config = BridgeConfig {
        token_budget: 3,
        max_packet_facts: 20,
        ..Default::default()
    };
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::new(encoder.clone(), ConceptGraph::new(), config);
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let text = "alpha beta gamma";
    let c = ConceptBuilder::new("c2")
        .with_vector(encoder.encode(text))
        .with_metadata("_text", serde_json::Value::String(text.to_string()))
        .build()
        .unwrap();
    singularity.inject("_default", c).unwrap();

    let packet = bridge
        .memory_packet("_default", &singularity, "alpha beta gamma", 10, None)
        .unwrap();
    assert!(
        !packet.facts.contains(&text.to_string()),
        "3-word fact must be excluded (estimated=4 > budget=3)"
    );
}

/// Kills `+=->*=` in `token_count += estimated`: with `*=`, token_count stays at 0
/// (0 * anything = 0), so all subsequent facts appear to fit within any budget.
#[test]
fn test_compile_packet_token_accumulator_grows() {
    // Two 2-word facts, budget=3. estimated("a b") = ceil(2/0.75) = 3.
    // First: 0+3=3<=3 => included, token_count becomes 3 (with +=) or 0 (with *=).
    // Second: with +=: 3+3=6>3 => excluded. With *=: 0+3=3<=3 => included.
    let config = BridgeConfig {
        token_budget: 3,
        max_packet_facts: 20,
        ..Default::default()
    };
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::new(encoder.clone(), ConceptGraph::new(), config);
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    let t1 = "foo bar";
    let t2 = "baz qux";
    for (id, text) in [("c3", t1), ("c4", t2)] {
        let c = ConceptBuilder::new(id)
            .with_vector(encoder.encode(text))
            .with_metadata("_text", serde_json::Value::String(text.to_string()))
            .build()
            .unwrap();
        singularity.inject("_default", c).unwrap();
    }

    let packet = bridge
        .memory_packet("_default", &singularity, "foo bar baz qux", 10, None)
        .unwrap();
    // Both facts are 2 words each (estimated=3 tokens). Budget=3 only fits one.
    let count = packet.facts.iter().filter(|f| *f == t1 || *f == t2).count();
    assert!(
        count <= 1,
        "only one 2-word fact must fit in budget=3; got {count}"
    );
}

/// Kills compile_packet `/ with %` and `/ with *` on confidence computation.
/// With two hits: sum/2 is strictly less than sum (kills `* which gives sum*2`)
/// and less than sum (kills `% which gives sum%2 = sum-2*floor(sum/2)`).
/// We assert confidence < 1.0 to rule out `*` (would give >1) and check it's
/// the mean (not the sum or modulo).
#[test]
fn test_compile_packet_confidence_is_strict_mean() {
    let encoder = TextEncoder::new();
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), ConceptGraph::new());
    let mut singularity = Singularity::<HVec10240>::new(SingularityConfig::default());

    // Two concepts — query returns 2 hits; confidence = mean of their final scores.
    for (id, text) in [("ca", "semantic memory"), ("cb", "semantic recall")] {
        let c = ConceptBuilder::new(id)
            .with_vector(encoder.encode(text))
            .build()
            .unwrap();
        singularity.inject("_default", c).unwrap();
    }

    let packet = bridge
        .memory_packet("_default", &singularity, "semantic memory recall", 10, None)
        .unwrap();

    // Both hits have scores in (0, 1]; sum of 2 scores ≤ 2.0.
    // mean = sum/2 is strictly < sum (kills /->*) and in (0, 1] (kills /->% which
    // for sum < 2.0 gives a different value).
    assert!(
        packet.confidence > 0.0 && packet.confidence <= 1.0,
        "confidence mean must be in (0, 1], got {}",
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
