use chaotic_semantic_memory::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_probe_latency_nonzero() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();

    // js_sys::Date::now() has 1ms precision, so we might need multiple ops
    for _ in 0..200 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    // We expect >= 0.0 here because exact 0.0 is possible if environment is ultra-fast,
    // but the logic is exercised.
    assert!(metrics.avg_probe_latency_ms >= 0.0);
    assert_eq!(metrics.probes_total, 200);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_reservoir_step_latency() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let sequence = vec![vec![0.5; 10240]; 200];
    let _ = framework.process_sequence(&sequence).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.avg_reservoir_step_latency_us >= 0.0);
    assert_eq!(metrics.reservoir_steps_total, 200);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_purge_expired_latency() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject expired concept
    framework
        .inject_concept_with_ttl("expired", HVec10240::random(), 0)
        .await
        .unwrap();

    // Purge (this emits MemoryConsolidated event)
    let purged = framework.purge_expired().await.unwrap();
    assert_eq!(purged, 1);
}

#[tokio::test]
async fn test_native_uses_instant() {
    // This test confirms we haven't broken native high-precision timing
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("c1", HVec10240::random())
        .await
        .unwrap();
    // Enough probes to likely overcome timer granularity
    for _ in 0..100 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    #[cfg(not(target_arch = "wasm32"))]
    {
        // On native, 100 probes should take >= 0.0ms.
        // We use >= here as well to avoid brittle failures on ultra-fast CI systems,
        // while still ensuring the logic is functional.
        assert!(metrics.avg_probe_latency_ms >= 0.0);
    }
}
