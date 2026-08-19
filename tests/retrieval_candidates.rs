#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Retrieval candidate generation coverage tests.
//!
//! Covers: RetrievalConfig getter, last_retrieval_stats, Singularity retrieval paths

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::{Singularity, SingularityConfig};

const NS: &str = "_default";

#[test]
fn singularity_retrieval_config_getter() {
    let sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = sing.retrieval_config();
    assert_eq!(config.max_candidates, 256);
    assert_eq!(config.bucket_probe_width, 2);
    assert!(config.bm25_absence_short_circuit);
    assert_eq!(config.early_exit_hdc, 0.92);
    assert_eq!(config.bridge_expand_cap, 16);
}

#[test]
fn singularity_last_retrieval_stats_default() {
    let sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let stats = sing.last_retrieval_stats(NS);
    assert_eq!(stats.candidate_count, 0);
    assert_eq!(stats.scored_count, 0);
}

#[test]
fn find_similar_populates_stats() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());

    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "find-stats-1".to_string(),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    let query = HVec10240::random();
    let _results = sing.find_similar(NS, &query, 5);

    // Stats should be populated (exact scan path)
    let stats = sing.last_retrieval_stats(NS);
    assert!(stats.scored_count > 0);
}

#[test]
fn find_similar_cached_populates_stats() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());

    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "cached-stats-1".to_string(),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    let query = HVec10240::random();
    let _results = sing.find_similar_cached(NS, &query, 5);

    let stats = sing.last_retrieval_stats(NS);
    assert!(stats.scored_count > 0);
}

#[test]
fn find_similar_empty_returns_empty_vec() {
    let sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let query = HVec10240::random();
    let results = sing.find_similar(NS, &query, 5);
    assert!(results.is_empty());
}

#[test]
fn find_similar_cached_empty_returns_empty_arc() {
    let sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let query = HVec10240::random();
    let results = sing.find_similar_cached(NS, &query, 5);
    assert!(results.is_empty());
}

#[test]
fn find_similar_with_associations() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());

    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    let v3 = HVec10240::random();

    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "assoc-1".to_string(),
            vector: v1,
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();
    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "assoc-2".to_string(),
            vector: v2,
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();
    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "assoc-3".to_string(),
            vector: v3,
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();

    // Create associations
    sing.associate(NS, "assoc-1", "assoc-2", 0.8).unwrap();
    sing.associate(NS, "assoc-2", "assoc-3", 0.6).unwrap();

    // Query with the first vector should find concepts
    let results = sing.find_similar(NS, &v1, 5);
    assert!(!results.is_empty());
}

#[test]
fn retrieval_config_with_graph_enabled() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = RetrievalConfig {
        enable_graph_candidates: true,
        graph_depth: 2,
        graph_fanout: 5,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();

    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "graph-1".to_string(),
            vector: v1,
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();
    sing.inject(
        NS,
        chaotic_semantic_memory::singularity::Concept {
            id: "graph-2".to_string(),
            vector: v2,
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        },
    )
    .unwrap();
    sing.associate(NS, "graph-1", "graph-2", 0.9).unwrap();

    // This should trigger graph candidate generation path
    let _results = sing.find_similar_cached(NS, &v1, 5);
    // Stats should reflect retrieval occurred
    let stats = sing.last_retrieval_stats(NS);
    assert!(stats.scored_count > 0);
}

#[test]
fn retrieval_config_with_bucket_enabled() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = RetrievalConfig {
        enable_bucket_candidates: true,
        bucket_probe_width: 4,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();

    // Inject multiple concepts
    for i in 0..10 {
        sing.inject(
            NS,
            chaotic_semantic_memory::singularity::Concept {
                id: format!("bucket-{i}"),
                vector: HVec10240::random(),
                metadata: std::collections::HashMap::new(),
                created_at: 1,
                modified_at: 1,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            },
        )
        .unwrap();
    }

    // This should trigger bucket candidate generation path
    let query = HVec10240::random();
    let _results = sing.find_similar_cached(NS, &query, 5);

    // Stats should reflect retrieval occurred
    let stats = sing.last_retrieval_stats(NS);
    assert!(stats.scored_count > 0);
}
