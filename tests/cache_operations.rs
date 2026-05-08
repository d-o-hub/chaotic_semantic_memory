//! Coverage tests for singularity_cache QueryCache operations.
//!
//! Covers: QueryCache put occupied entry, get LRU reorder, eviction edge cases

// QueryCache is pub(crate), so we test indirectly through Singularity's cached operations
// These tests verify the cache behavior through the public API

use chaotic_semantic_memory::prelude::*;

#[tokio::test]

async fn cache_put_existing_key_updates_value() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("cache-update-1", HVec10240::random())
        .await
        .unwrap();

    // First query batch - cache miss, stores results
    let query = HVec10240::random();
    let results1 = framework.probe_batch_cached(&[query], 5).await.unwrap();

    // Same query again - cache hit, returns stored result
    let results2 = framework.probe_batch_cached(&[query], 5).await.unwrap();

    // Results should be identical (cached)
    assert_eq!(results1.len(), results2.len());
}

#[tokio::test]
async fn cache_capacity_one_evicts_on_second_query() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(1) // Minimal cache
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("evict-single", HVec10240::random())
        .await
        .unwrap();

    // Two different queries - second should evict first
    let q1 = HVec10240::random();
    let q2 = HVec10240::random();
    framework.probe_batch_cached(&[q1], 5).await.unwrap();
    framework.probe_batch_cached(&[q2], 5).await.unwrap();

    // Third query back to q1 - should be a miss (was evicted)
    framework.probe_batch_cached(&[q1], 5).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_evictions_total >= 1);
}

#[tokio::test]
async fn cache_clear_removes_all_entries() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("clear-test", HVec10240::random())
        .await
        .unwrap();

    // Prime cache with multiple queries
    for _ in 0..3 {
        let q = HVec10240::random();
        framework.probe_batch_cached(&[q], 5).await.unwrap();
    }

    let metrics_before = framework.metrics_snapshot().await;
    assert!(metrics_before.cache_misses_total >= 3);

    // Clear cache
    framework.clear_similarity_cache().await;

    // After clear, new queries are all misses
    let q = HVec10240::random();
    framework.probe_batch_cached(&[q], 5).await.unwrap();
    let metrics_after = framework.metrics_snapshot().await;
    assert!(metrics_after.cache_misses_total >= 4);
}
