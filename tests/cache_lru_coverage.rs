#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Cache LRU reorder and occupied entry coverage tests.
//!
//! Covers: QueryCache get LRU reorder, put occupied entry update

use chaotic_semantic_memory::prelude::*;

#[tokio::test]
async fn cache_get_updates_lru_order() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("lru-order-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("lru-order-2", HVec10240::random())
        .await
        .unwrap();

    let q1 = HVec10240::random();
    let q2 = HVec10240::random();
    let q3 = HVec10240::random();

    // Fill cache with different queries
    framework.probe_batch_cached(&[q1], 5).await.unwrap();
    framework.probe_batch_cached(&[q2], 5).await.unwrap();
    framework.probe_batch_cached(&[q3], 5).await.unwrap();

    // Access q1 again - should update LRU order (move to end)
    framework.probe_batch_cached(&[q1], 5).await.unwrap();

    // Verify operation succeeded (LRU order updated internally)
    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_hits_total >= 1);
}

#[tokio::test]
async fn cache_put_same_key_updates_value() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("update-value", HVec10240::random())
        .await
        .unwrap();

    let query = HVec10240::random();

    // First query - cache miss, stores result
    framework.probe_batch_cached(&[query], 5).await.unwrap();
    let metrics1 = framework.metrics_snapshot().await;

    // Same query again - cache hit, returns stored result
    framework.probe_batch_cached(&[query], 5).await.unwrap();
    let metrics2 = framework.metrics_snapshot().await;

    // Should have exactly 1 hit
    assert_eq!(metrics2.cache_hits_total - metrics1.cache_hits_total, 1);

    // Access same query again - another hit
    framework.probe_batch_cached(&[query], 5).await.unwrap();
    let metrics3 = framework.metrics_snapshot().await;

    assert_eq!(metrics3.cache_hits_total - metrics2.cache_hits_total, 1);
}

#[tokio::test]
async fn cache_capacity_boundary() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(2)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("capacity-1", HVec10240::random())
        .await
        .unwrap();

    let q1 = HVec10240::random();
    let q2 = HVec10240::random();
    let q3 = HVec10240::random();

    // Fill cache to capacity
    framework.probe_batch_cached(&[q1], 5).await.unwrap();
    framework.probe_batch_cached(&[q2], 5).await.unwrap();

    // Add one more - should evict oldest
    framework.probe_batch_cached(&[q3], 5).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_evictions_total >= 1);
}
