//! Retrieval candidate generation coverage tests.
//!
//! Covers: RetrievalConfig getter, last_retrieval_stats, Singularity retrieval paths

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::Singularity;

#[test]
fn singularity_retrieval_config_getter() {
    let sing = Singularity::new();
    let config = sing.retrieval_config();
    assert_eq!(config.max_candidates, 1000);
    assert_eq!(config.bucket_probe_width, 2);
}

#[test]
fn singularity_last_retrieval_stats_default() {
    let sing = Singularity::new();
    let stats = sing.last_retrieval_stats();
    assert_eq!(stats.candidate_count, 0);
    assert_eq!(stats.scored_count, 0);
}

#[test]
fn find_similar_populates_stats() {
    let mut sing = Singularity::new();

    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "find-stats-1".to_string(),
        vector: HVec10240::random(),
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();

    let query = HVec10240::random();
    let _results = sing.find_similar(&query, 5);

    // Stats should be populated (exact scan path)
    let stats = sing.last_retrieval_stats();
    assert!(stats.scored_count > 0);
}

#[test]
fn find_similar_cached_populates_stats() {
    let mut sing = Singularity::new();

    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "cached-stats-1".to_string(),
        vector: HVec10240::random(),
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();

    let query = HVec10240::random();
    let _results = sing.find_similar_cached(&query, 5);

    let stats = sing.last_retrieval_stats();
    assert!(stats.scored_count > 0);
}

#[test]
fn find_similar_empty_returns_empty_vec() {
    let sing = Singularity::new();
    let query = HVec10240::random();
    let results = sing.find_similar(&query, 5);
    assert!(results.is_empty());
}

#[test]
fn find_similar_cached_empty_returns_empty_arc() {
    let sing = Singularity::new();
    let query = HVec10240::random();
    let results = sing.find_similar_cached(&query, 5);
    assert!(results.is_empty());
}

#[test]
fn find_similar_with_associations() {
    let mut sing = Singularity::new();

    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    let v3 = HVec10240::random();

    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "assoc-1".to_string(),
        vector: v1,
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();
    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "assoc-2".to_string(),
        vector: v2,
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();
    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "assoc-3".to_string(),
        vector: v3,
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();

    // Create associations
    sing.associate("assoc-1", "assoc-2", 0.8).unwrap();
    sing.associate("assoc-2", "assoc-3", 0.6).unwrap();

    // Query with the first vector should find concepts
    let results = sing.find_similar(&v1, 5);
    assert!(!results.is_empty());
}

#[test]
fn retrieval_config_with_graph_enabled() {
    let mut sing = Singularity::new();
    let config = RetrievalConfig {
        enable_graph_candidates: true,
        graph_depth: 2,
        graph_fanout: 5,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();

    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "graph-1".to_string(),
        vector: v1,
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();
    sing.inject(chaotic_semantic_memory::singularity::Concept {
        id: "graph-2".to_string(),
        vector: v2,
        metadata: std::collections::HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    })
    .unwrap();
    sing.associate("graph-1", "graph-2", 0.9).unwrap();

    // This should trigger graph candidate generation path
    let _results = sing.find_similar_cached(&v1, 5);
    // Stats should reflect retrieval occurred
    let stats = sing.last_retrieval_stats();
    assert!(stats.scored_count > 0);
}

#[test]
fn retrieval_config_with_bucket_enabled() {
    let mut sing = Singularity::new();
    let config = RetrievalConfig {
        enable_bucket_candidates: true,
        bucket_probe_width: 4,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();

    // Inject multiple concepts
    for i in 0..10 {
        sing.inject(chaotic_semantic_memory::singularity::Concept {
            id: format!("bucket-{}", i),
            vector: HVec10240::random(),
            metadata: std::collections::HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        })
        .unwrap();
    }

    // This should trigger bucket candidate generation path
    let query = HVec10240::random();
    let _results = sing.find_similar_cached(&query, 5);

    // Stats should reflect retrieval occurred
    let stats = sing.last_retrieval_stats();
    assert!(stats.scored_count > 0);
}
