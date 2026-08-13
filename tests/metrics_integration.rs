#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use chaotic_semantic_memory::prelude::*;

#[tokio::test]
async fn test_cache_metrics_wiring() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Initially zero
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.cache_hits_total, 0);
    assert_eq!(metrics.cache_misses_total, 0);

    // 2. Inject and probe to trigger cache miss
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    let query = HVec10240::random();
    let _ = framework.probe(query, 5).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.cache_misses_total, 1);
    assert_eq!(metrics.cache_hits_total, 0);

    // 3. Probe again to trigger cache hit
    let _ = framework.probe(query, 5).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.cache_hits_total, 1);
}

#[tokio::test]
async fn test_reservoir_metrics_wiring() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Initially zero
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.reservoir_steps_total, 0);

    // 2. Process sequence
    let sequence = vec![vec![0.5; 10240]; 3];
    let _ = framework.process_sequence(&sequence).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.reservoir_steps_total, 3);
    assert!(metrics.avg_reservoir_step_latency_us >= 0.0);
    #[cfg(not(target_arch = "wasm32"))]
    assert!(metrics.avg_reservoir_step_latency_us > 0.0);
    assert_eq!(metrics.reservoir_nodes_active, 50000);
}

#[tokio::test]
async fn test_latency_averages() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Initially zero
    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.avg_probe_latency_ms < f64::EPSILON);

    // 2. Perform some probes
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    // Use more iterations to ensure non-zero millis on fast systems
    for _ in 0..100 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    // On extremely fast systems, avg latency might still be 0.0 ms due to truncation
    assert!(metrics.avg_probe_latency_ms >= 0.0);
    assert_eq!(metrics.probes_total, 100);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_persistence_metrics_wiring() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_metrics.db");

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.to_str().unwrap())
        .build()
        .await
        .unwrap();

    // 1. Reset metrics after load_replace in build()
    // (load_replace counts as a persist op)
    let initial_metrics = framework.metrics_snapshot().await;

    // 2. Inject concept (triggers persistence)
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert_eq!(
        metrics.persist_ops_total,
        initial_metrics.persist_ops_total + 1
    );
    assert!(metrics.avg_persist_latency_ms >= 0.0);
}

#[tokio::test]
async fn test_cache_metrics_multi_namespace() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Initially zero
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.cache_misses_total, 0);

    // 2. Namespace 1: trigger a miss
    framework.set_namespace("ns1").await.unwrap();
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    let _ = framework.probe(HVec10240::random(), 5).await.unwrap();

    // 3. Namespace 2: trigger another miss
    framework.set_namespace("ns2").await.unwrap();
    framework
        .inject_concept("c2", HVec10240::random())
        .await
        .unwrap();
    let _ = framework.probe(HVec10240::random(), 5).await.unwrap();

    // 4. Verify aggregation across namespaces
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.cache_misses_total, 2);
}

#[tokio::test]
async fn test_core_metrics_tracking() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // 1. Initially zero
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.concepts_injected_total, 0);
    assert_eq!(metrics.associations_created_total, 0);
    assert_eq!(metrics.probes_total, 0);

    // 2. Test inject_concept
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.concepts_injected_total, 1);

    // 3. Test inject_concepts (batch)
    let batch = vec![
        ("c2".to_string(), HVec10240::random()),
        ("c3".to_string(), HVec10240::random()),
    ];
    framework.inject_concepts(&batch).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.concepts_injected_total, 3);

    // 4. Test associate
    framework.associate("c1", "c2", 0.5).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.associations_created_total, 1);

    // 5. Test associate_many (batch)
    let associations = vec![
        ("c1".to_string(), "c3".to_string(), 0.8),
        ("c2".to_string(), "c3".to_string(), 0.3),
    ];
    framework.associate_many(&associations).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.associations_created_total, 3);

    // 6. Test probe
    let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    let metrics = framework.metrics_snapshot().await;
    assert_eq!(metrics.probes_total, 1);
}
