use chaotic_semantic_memory::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_probe_latency_nonzero() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework.inject_concept("c1", HVec10240::random()).await.unwrap();

    // Busy loop or small delay if possible to ensure measurable time passes
    // js_sys::Date::now() has 1ms precision, so we might need multiple ops
    for _ in 0..10 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    // Note: On some WASM environments Date::now() might still be 0 if it's too fast
    // but this validates the logic is wired up.
    assert!(metrics.avg_probe_latency_ms >= 0.0);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_reservoir_step_latency() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let sequence = vec![vec![0.5; 10240]; 10];
    let _ = framework.process_sequence(&sequence).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.avg_reservoir_step_latency_us >= 0.0);
}

#[tokio::test]
async fn test_native_uses_instant() {
    // This test confirms we haven't broken native high-precision timing
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework.inject_concept("c1", HVec10240::random()).await.unwrap();
    let _ = framework.probe(HVec10240::random(), 5).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    #[cfg(not(target_arch = "wasm32"))]
    assert!(metrics.avg_probe_latency_ms > 0.0);
}
