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
    let _ = framework.probe(query.clone(), 5).await.unwrap();

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
    assert_eq!(metrics.avg_probe_latency_ms, 0.0);

    // 2. Perform some probes
    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    for _ in 0..5 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.avg_probe_latency_ms >= 0.0);
    assert_eq!(metrics.probes_total, 5);
}

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
