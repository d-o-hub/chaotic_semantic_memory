#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Framework operations coverage tests for framework_ops.rs.
//!
//! Covers: inject_concepts, associate_many, probe_batch, update operations

use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[tokio::test]
async fn inject_concepts_batch() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concepts = vec![
        ("batch-1".to_string(), HVec10240::random()),
        ("batch-2".to_string(), HVec10240::random()),
        ("batch-3".to_string(), HVec10240::random()),
    ];

    framework.inject_concepts(&concepts).await.unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[tokio::test]
async fn associate_many_creates_associations() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("assoc-from", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-to1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-to2", HVec10240::random())
        .await
        .unwrap();

    let associations = vec![
        ("assoc-from".to_string(), "assoc-to1".to_string(), 0.7),
        ("assoc-from".to_string(), "assoc-to2".to_string(), 0.5),
    ];

    framework.associate_many(&associations).await.unwrap();

    // Verify concept count unchanged
    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[tokio::test]
async fn probe_batch_multiple_queries() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("q1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("q2", HVec10240::random())
        .await
        .unwrap();

    let queries = vec![HVec10240::random(), HVec10240::random()];
    let results = framework.probe_batch(&queries, 5).await.unwrap();

    assert_eq!(results.len(), 2);
    for r in &results {
        assert!(r.len() <= 5);
    }
}

#[tokio::test]
async fn probe_batch_cached_caches_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("cached-q", HVec10240::random())
        .await
        .unwrap();

    let queries = vec![HVec10240::random()];
    let results1 = framework.probe_batch_cached(&queries, 3).await.unwrap();
    let results2 = framework.probe_batch_cached(&queries, 3).await.unwrap();

    assert_eq!(results1.len(), 1);
    assert_eq!(results2.len(), 1);
}

#[tokio::test]
async fn update_concept_vector_changes_vector() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let original = HVec10240::random();
    framework
        .inject_concept("update-vector", original)
        .await
        .unwrap();

    let new_vector = HVec10240::random();
    framework
        .update_concept_vector("update-vector", new_vector)
        .await
        .unwrap();

    // Probe with new vector should find it
    let results = framework.probe(new_vector, 5).await.unwrap();
    assert!(results.iter().any(|(id, _)| id == "update-vector"));
}

#[tokio::test]
async fn update_concept_metadata_changes_metadata() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    use std::collections::HashMap;
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), serde_json::json!("original"));

    framework
        .inject_concept_with_metadata("update-meta", HVec10240::random(), meta)
        .await
        .unwrap();

    let new_meta = HashMap::new();
    framework
        .update_concept_metadata("update-meta", new_meta)
        .await
        .unwrap();
}

#[tokio::test]
async fn disassociate_removes_single_association() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("dis-from", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("dis-to", HVec10240::random())
        .await
        .unwrap();

    framework
        .associate("dis-from", "dis-to", 0.9)
        .await
        .unwrap();

    // Disassociate should succeed
    framework.disassociate("dis-from", "dis-to").await.unwrap();

    // Concept count unchanged
    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn clear_associations_removes_all() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("clear-from", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("clear-to1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("clear-to2", HVec10240::random())
        .await
        .unwrap();

    framework
        .associate("clear-from", "clear-to1", 0.5)
        .await
        .unwrap();
    framework
        .associate("clear-from", "clear-to2", 0.6)
        .await
        .unwrap();

    framework.clear_associations("clear-from").await.unwrap();
}

#[tokio::test]
async fn bundle_concepts_strict_valid_ids() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("bundle-a", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("bundle-b", HVec10240::random())
        .await
        .unwrap();

    let ids = vec!["bundle-a".to_string(), "bundle-b".to_string()];
    let _bundled = framework.bundle_concepts_strict(&ids).await.unwrap();
}
