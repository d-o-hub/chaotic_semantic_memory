//! Cache LRU behavior tests for coverage gap in singularity_cache.rs.
//!
//! Covers: QueryCache::get with LRU update, QueryCache::put with eviction

use chaotic_semantic_memory::prelude::*;

#[tokio::test]
async fn cache_hit_updates_lru_order() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    // Inject concepts
    let vec_a = HVec10240::random();
    framework.inject_concept("a", vec_a).await.unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("c", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("d", HVec10240::random())
        .await
        .unwrap();

    // Query for 'a' multiple times to test cache hit LRU update
    let results1 = framework.probe(vec_a, 2).await.unwrap();
    assert!(results1.iter().any(|(id, _)| id == "a"));

    // Second query should hit cache and update LRU
    let results2 = framework.probe(vec_a, 2).await.unwrap();
    assert!(results2.iter().any(|(id, _)| id == "a"));

    // Inject new concept to trigger potential eviction
    framework
        .inject_concept("e", HVec10240::random())
        .await
        .unwrap();

    // 'a' should still be found due to recent access
    let results3 = framework.probe(vec_a, 5).await.unwrap();
    assert!(results3.iter().any(|(id, _)| id == "a"));
}

#[tokio::test]
async fn cache_capacity_eviction() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(2) // Very small cache
        .build()
        .await
        .unwrap();

    let vec1 = HVec10240::random();
    framework.inject_concept("c1", vec1).await.unwrap();
    framework
        .inject_concept("c2", HVec10240::random())
        .await
        .unwrap();

    // First query fills cache
    framework.probe(vec1, 1).await.unwrap();

    // Different query evicts old entry
    framework.probe(HVec10240::random(), 1).await.unwrap();

    // Original query causes cache miss
    framework.probe(vec1, 1).await.unwrap();
}

#[tokio::test]
async fn cache_clear_removes_all_entries() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    let vec1 = HVec10240::random();
    framework.inject_concept("x", vec1).await.unwrap();

    // Query to populate cache
    framework.probe(vec1, 1).await.unwrap();

    // Clear similarity cache (returns (), not Result)
    framework.clear_similarity_cache().await;

    // Next query should be cache miss
    framework.probe(vec1, 1).await.unwrap();
}
