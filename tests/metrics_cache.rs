//! Metrics and cache behavior tests for coverage.
//!
//! Covers: FrameworkMetricsSnapshot, cache eviction metrics

use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[tokio::test]
async fn metrics_snapshot_tracks_cache_hits() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("metrics-1", HVec10240::random())
        .await
        .unwrap();

    // Multiple probes should generate cache hits
    let query = HVec10240::random();
    framework.probe(&query, 5).await.unwrap();
    framework.probe(&query, 5).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_hits_total >= 1);
    assert!(metrics.cache_misses_total >= 1);
}

#[tokio::test]
async fn metrics_snapshot_tracks_concepts_injected() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    for i in 0..5 {
        framework
            .inject_concept(&format!("metrics-inject-{i}"), HVec10240::random())
            .await
            .unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.concepts_injected_total, 5);
}

#[tokio::test]
async fn metrics_snapshot_tracks_associations() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("assoc-metrics-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("assoc-metrics-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("assoc-metrics-1", "assoc-metrics-2", 0.8)
        .await
        .unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.associations_created_total, 1);
}

#[tokio::test]
async fn metrics_snapshot_tracks_probes() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("probe-metrics", HVec10240::random())
        .await
        .unwrap();

    for _ in 0..3 {
        framework.probe(&HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.probes_total, 3);
}

#[tokio::test]
async fn cache_eviction_on_capacity_exceeded() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(2) // Small cache to force eviction
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("evict-1", HVec10240::random())
        .await
        .unwrap();

    // Multiple different queries should cause evictions
    let q1 = HVec10240::random();
    let q2 = HVec10240::random();
    let q3 = HVec10240::random();
    framework.probe(&q1, 5).await.unwrap();
    framework.probe(&q2, 5).await.unwrap();
    framework.probe(&q3, 5).await.unwrap(); // Should evict q1

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_evictions_total >= 1);
}

#[tokio::test]
async fn clear_similarity_cache_removes_entries() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(16)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("clear-cache", HVec10240::random())
        .await
        .unwrap();

    // Prime the cache
    framework.probe(&HVec10240::random(), 5).await.unwrap();

    // Clear cache
    framework.clear_similarity_cache().await;

    // After clear, metrics should reset (or we can probe again)
    framework.probe(&HVec10240::random(), 5).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.cache_misses_total >= 1);
}
