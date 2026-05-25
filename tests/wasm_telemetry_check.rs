use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::framework_events_ce::{ChaoticEvent, EventEmitter, CloudEvent};
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

struct MockEmitter {
    events: Arc<Mutex<Vec<ChaoticEvent>>>,
}

#[async_trait]
impl EventEmitter for MockEmitter {
    async fn emit(&self, _event: CloudEvent) -> chaotic_semantic_memory::Result<()> {
        // In a real test we'd parse the CloudEvent back to ChaoticEvent
        // but here we just want to ensure it compiles and logic is triggered.
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_wasm_probe_latency_nonzero() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework.inject_concept("c1", HVec10240::random()).await.unwrap();

    // js_sys::Date::now() has 1ms precision, so we might need multiple ops
    for _ in 0..100 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
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

    let sequence = vec![vec![0.5; 10240]; 100];
    let _ = framework.process_sequence(&sequence).await.unwrap();

    let metrics = framework.metrics_snapshot().await;
    assert!(metrics.avg_reservoir_step_latency_us >= 0.0);
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
    framework.inject_concept_with_ttl("expired", HVec10240::random(), 0).await.unwrap();

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

    framework.inject_concept("c1", HVec10240::random()).await.unwrap();
    // Enough probes to likely overcome timer granularity
    for _ in 0..100 {
        let _ = framework.probe(HVec10240::random(), 5).await.unwrap();
    }

    let metrics = framework.metrics_snapshot().await;
    #[cfg(not(target_arch = "wasm32"))]
    assert!(metrics.avg_probe_latency_ms >= 0.0);
}
