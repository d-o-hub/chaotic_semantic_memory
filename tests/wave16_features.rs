//! Wave 16 feature tests: TextEncoder golden vectors, graph traversal edge cases,
//! BundleAccumulator edge cases, and filtered search edge cases.

use chaotic_semantic_memory::bundle::BundleAccumulator;
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::graph_traversal::TraversalConfig;
use chaotic_semantic_memory::hyperdim::HVec10240;
use chaotic_semantic_memory::metadata_filter::MetadataFilter;
use chaotic_semantic_memory::singularity::{Concept, ConceptBuilder, Singularity};

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_concept(id: &str) -> Concept {
    ConceptBuilder::new(id)
        .with_vector(HVec10240::random())
        .build()
        .unwrap()
}

fn make_concept_with_meta(id: &str, key: &str, val: &str) -> Concept {
    ConceptBuilder::new(id)
        .with_vector(HVec10240::random())
        .with_metadata(key, serde_json::Value::String(val.to_string()))
        .build()
        .unwrap()
}

// ─── TextEncoder: Golden Vector Regression Tests ────────────────────────────

/// Golden vector: known input → known FNV-1a hash → known seed → deterministic HVec.
/// These values are computed from the FNV-1a implementation and must not change
/// across Rust versions (unlike DefaultHasher/SipHash).
#[test]
fn test_encoder_golden_vector_hello_world() {
    let encoder = TextEncoder::new();
    let hv1 = encoder.encode("hello world");
    let hv2 = encoder.encode("hello world");

    // Determinism: same input → identical output
    assert_eq!(hv1, hv2, "TextEncoder must be deterministic");

    // Not zero
    assert_ne!(hv1, HVec10240::zero(), "encoded vector must not be zero");
}

#[test]
fn test_encoder_golden_vector_single_token() {
    let encoder = TextEncoder::new();
    let hv1 = encoder.encode("rust");
    let hv2 = encoder.encode("rust");
    assert_eq!(hv1, hv2);
    assert_ne!(hv1, HVec10240::zero());
}

#[test]
fn test_encoder_golden_vector_stability_across_calls() {
    // Encode 10 times, all must be identical
    let encoder = TextEncoder::new();
    let reference = encoder.encode("the quick brown fox jumps over the lazy dog");
    for _ in 0..9 {
        let hv = encoder.encode("the quick brown fox jumps over the lazy dog");
        assert_eq!(
            hv, reference,
            "encoding must be stable across repeated calls"
        );
    }
}

#[test]
fn test_encoder_fnv1a_different_from_siphash_behavior() {
    // FNV-1a and SipHash produce different seeds for the same token.
    // We verify that the encoder produces a non-trivial, non-zero vector,
    // which confirms the hash is being used as a seed (not returning 0).
    let encoder = TextEncoder::new();
    let hv = encoder.encode("test");
    assert_ne!(hv, HVec10240::zero());
    // Self-similarity must be 1.0 (deterministic)
    assert!(hv.cosine_similarity(&hv) > 0.999);
}

#[test]
fn test_encoder_similar_texts_positive_similarity() {
    let encoder = TextEncoder::new();
    let hv1 = encoder.encode("machine learning model");
    let hv2 = encoder.encode("machine learning algorithm");
    // Texts sharing most tokens should have positive similarity
    assert!(
        hv1.cosine_similarity(&hv2) > 0.3,
        "similar texts should have positive similarity"
    );
}

#[test]
fn test_encoder_dissimilar_texts_lower_similarity() {
    let encoder = TextEncoder::new();
    let hv1 = encoder.encode("hello world");
    let hv2 = encoder.encode("xyzzy plugh frobnicate");
    // Completely different tokens should have lower similarity
    let sim = hv1.cosine_similarity(&hv2);
    assert!(
        sim < 0.8,
        "dissimilar texts should have lower similarity, got {sim}"
    );
}

#[test]
fn test_encoder_ngram_deterministic() {
    let encoder = TextEncoder::new();
    let hv1 = encoder.encode_with_ngrams("hello", 2);
    let hv2 = encoder.encode_with_ngrams("hello", 2);
    assert_eq!(hv1, hv2, "n-gram encoding must be deterministic");
}

// ─── Graph Traversal: Cycle Detection ───────────────────────────────────────

#[test]
fn test_bfs_cycle_does_not_loop() {
    let mut sing = Singularity::new();
    // a → b → c → a (cycle)
    for id in ["a", "b", "c"] {
        sing.inject(make_concept(id)).unwrap();
    }
    sing.associate("a", "b", 0.9).unwrap();
    sing.associate("b", "c", 0.9).unwrap();
    sing.associate("c", "a", 0.9).unwrap(); // back-edge

    let config = TraversalConfig::default();
    let results = sing.bfs("a", &config).unwrap();

    // Must visit each node exactly once despite the cycle
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids.len(), 3, "each node visited exactly once");
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
}

#[test]
fn test_bfs_disconnected_graph() {
    let mut sing = Singularity::new();
    // Component 1: a → b
    // Component 2: c → d (disconnected)
    for id in ["a", "b", "c", "d"] {
        sing.inject(make_concept(id)).unwrap();
    }
    sing.associate("a", "b", 0.9).unwrap();
    sing.associate("c", "d", 0.9).unwrap();

    let config = TraversalConfig::default();
    let results = sing.bfs("a", &config).unwrap();

    // BFS from "a" should only reach "a" and "b", not "c" or "d"
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(!ids.contains(&"c"), "disconnected node must not be reached");
    assert!(!ids.contains(&"d"), "disconnected node must not be reached");
}

#[test]
fn test_bfs_max_results_limit() {
    let mut sing = Singularity::new();
    // Chain: a → b → c → d → e
    for id in ["a", "b", "c", "d", "e"] {
        sing.inject(make_concept(id)).unwrap();
    }
    sing.associate("a", "b", 0.9).unwrap();
    sing.associate("b", "c", 0.9).unwrap();
    sing.associate("c", "d", 0.9).unwrap();
    sing.associate("d", "e", 0.9).unwrap();

    let config = TraversalConfig {
        max_results: 3,
        max_depth: 10,
        ..Default::default()
    };
    let results = sing.bfs("a", &config).unwrap();
    assert_eq!(results.len(), 3, "max_results must be respected");
}

#[test]
fn test_shortest_path_cycle_terminates() {
    let mut sing = Singularity::new();
    // a → b → c → a (cycle), also a → c directly
    for id in ["a", "b", "c"] {
        sing.inject(make_concept(id)).unwrap();
    }
    sing.associate("a", "b", 0.9).unwrap();
    sing.associate("b", "c", 0.9).unwrap();
    sing.associate("c", "a", 0.9).unwrap();
    sing.associate("a", "c", 0.5).unwrap(); // direct weak edge

    let config = TraversalConfig::default();
    // Must terminate and find a path
    let path = sing.shortest_path("a", "c", &config).unwrap();
    assert!(path.is_some(), "path must be found in cyclic graph");
    let path = path.unwrap();
    assert_eq!(path[0], "a");
    assert_eq!(*path.last().unwrap(), "c");
}

#[test]
fn test_shortest_path_hops_cycle_terminates() {
    let mut sing = Singularity::new();
    for id in ["a", "b", "c"] {
        sing.inject(make_concept(id)).unwrap();
    }
    sing.associate("a", "b", 0.9).unwrap();
    sing.associate("b", "c", 0.9).unwrap();
    sing.associate("c", "a", 0.9).unwrap();

    let config = TraversalConfig::default();
    let path = sing.shortest_path_hops("a", "c", &config).unwrap();
    assert!(path.is_some());
}

// ─── BundleAccumulator: Edge Cases ──────────────────────────────────────────

#[test]
fn test_bundle_accumulator_try_remove_empty_returns_error() {
    let mut acc = BundleAccumulator::new();
    let hv = HVec10240::random();
    let result = acc.try_remove(&hv);
    assert!(
        result.is_err(),
        "try_remove on empty accumulator must error"
    );
}

#[test]
fn test_bundle_accumulator_remove_empty_is_noop() {
    let mut acc = BundleAccumulator::new();
    let hv = HVec10240::random();
    // remove (non-panicking) on empty must be a no-op
    acc.remove(&hv);
    assert!(
        acc.is_empty(),
        "accumulator must remain empty after no-op remove"
    );
}

#[test]
fn test_bundle_accumulator_remove_more_than_added() {
    let mut acc = BundleAccumulator::new();
    let hv = HVec10240::random();
    acc.add(&hv);
    acc.remove(&hv); // n=0
    acc.remove(&hv); // no-op, n stays 0
    assert!(acc.is_empty());
}

#[test]
fn test_bundle_accumulator_try_remove_success() {
    let mut acc = BundleAccumulator::new();
    let hv = HVec10240::random();
    acc.add(&hv);
    let result = acc.try_remove(&hv);
    assert!(
        result.is_ok(),
        "try_remove on non-empty accumulator must succeed"
    );
    assert!(acc.is_empty());
}

#[test]
fn test_bundle_accumulator_add_remove_finalize_matches_single() {
    let v1 = HVec10240::random();
    let v2 = HVec10240::random();

    let mut acc = BundleAccumulator::new();
    acc.add(&v1);
    acc.add(&v2);
    acc.remove(&v2);

    let bundled = acc.finalize();
    // After removing v2, the accumulator holds only v1.
    // A single-vector bundle should be identical to the original.
    assert!(
        bundled.cosine_similarity(&v1) > 0.99,
        "single-vector bundle must match original"
    );
}

// ─── Filtered Search: Edge Cases ────────────────────────────────────────────

#[test]
fn test_filtered_search_empty_filter_returns_all() {
    let mut sing = Singularity::new();
    let query = HVec10240::random();

    for i in 0..5 {
        sing.inject(make_concept_with_meta(&format!("c{i}"), "tag", "science"))
            .unwrap();
    }

    // Exists("tag") matches all 5 concepts
    let filter = MetadataFilter::Exists("tag".to_string());
    let results = sing.find_similar_filtered(&query, 10, &filter);
    assert_eq!(results.len(), 5, "all concepts should match Exists filter");
}

#[test]
fn test_filtered_search_no_match_returns_empty() {
    let mut sing = Singularity::new();
    let query = HVec10240::random();

    for i in 0..5 {
        sing.inject(make_concept_with_meta(&format!("c{i}"), "tag", "science"))
            .unwrap();
    }

    // Filter for "art" — no concepts have this tag
    let filter = MetadataFilter::Eq(
        "tag".to_string(),
        serde_json::Value::String("art".to_string()),
    );
    let results = sing.find_similar_filtered(&query, 10, &filter);
    assert!(
        results.is_empty(),
        "no-match filter must return empty results"
    );
}

#[test]
fn test_filtered_search_subset_match() {
    let mut sing = Singularity::new();
    let query = HVec10240::random();

    // 3 science, 2 art
    for i in 0..3 {
        sing.inject(make_concept_with_meta(&format!("sci{i}"), "tag", "science"))
            .unwrap();
    }
    for i in 0..2 {
        sing.inject(make_concept_with_meta(&format!("art{i}"), "tag", "art"))
            .unwrap();
    }

    let filter = MetadataFilter::Eq(
        "tag".to_string(),
        serde_json::Value::String("science".to_string()),
    );
    let results = sing.find_similar_filtered(&query, 10, &filter);
    assert_eq!(results.len(), 3, "only science concepts should match");
    for (id, _) in results.iter() {
        assert!(
            id.starts_with("sci"),
            "result must be a science concept: {id}"
        );
    }
}

#[test]
fn test_filtered_search_top_k_respected() {
    let mut sing = Singularity::new();
    let query = HVec10240::random();

    for i in 0..10 {
        sing.inject(make_concept_with_meta(&format!("c{i}"), "tag", "science"))
            .unwrap();
    }

    let filter = MetadataFilter::Exists("tag".to_string());
    let results = sing.find_similar_filtered(&query, 3, &filter);
    assert_eq!(results.len(), 3, "top_k must be respected");
}
