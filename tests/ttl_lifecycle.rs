#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! TTL (Time-To-Live) lifecycle tests for coverage gap in framework_ttl.rs.
//!
//! Covers: inject_concept_with_ttl, inject_text_with_ttl, purge_expired

use chaotic_semantic_memory::prelude::*;

const NS: &str = "_default";

#[tokio::test]
async fn inject_concept_with_ttl_stores_concept() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let vector = HVec10240::random();
    framework
        .inject_concept_with_ttl("ttl-concept", vector, 3600)
        .await
        .unwrap();

    let results = framework.probe(vector, 10).await.unwrap();
    assert!(results.iter().any(|(id, _)| id == "ttl-concept"));
}

#[tokio::test]
async fn inject_text_with_ttl_stores_concept() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_text_with_ttl("ttl-text", "test content", 3600)
        .await
        .unwrap();

    let results = framework.probe_text("test", 10).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn purge_expired_removes_expired_concepts() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject with very short TTL (1 second)
    let vector = HVec10240::random();
    framework
        .inject_concept_with_ttl("short-ttl", vector, 1)
        .await
        .unwrap();

    // Inject a concept with long TTL (should not be purged)
    framework
        .inject_concept_with_ttl("long-ttl", HVec10240::random(), 3600)
        .await
        .unwrap();

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Purge expired concepts
    let purged = framework.purge_expired().await.unwrap();
    assert!(purged >= 1, "At least one concept should be purged");

    // Verify long-ttl still exists
    let results = framework.probe(HVec10240::random(), 10).await.unwrap();
    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.contains(&"long-ttl"), "Long TTL concept should remain");
}

#[tokio::test]
async fn inject_text_with_metadata_works() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    use std::collections::HashMap;
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), serde_json::json!("test"));

    framework
        .inject_text_with_metadata("meta-text", "content", metadata)
        .await
        .unwrap();

    // Verify concept exists via probe
    let results = framework.probe_text("content", 5).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn probe_text_encodes_and_searches() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_text("doc1", "machine learning algorithms")
        .await
        .unwrap();
    framework
        .inject_text("doc2", "neural network architecture")
        .await
        .unwrap();

    let results = framework.probe_text("learning", 5).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn shutdown_cleanup_returns_promptly_for_running_loop() {
    // Owned lifecycle: an enabled cleanup loop must be stoppable via the
    // public shutdown API within its 5s bound.
    let mut ttl_config = chaotic_semantic_memory::framework_ttl_advanced::TtlConfig::default();
    ttl_config.cleanup_interval_seconds = 1;
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_ttl_config(ttl_config)
        .build()
        .await
        .unwrap();

    // Let the loop tick at least once, then stop it.
    tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
    let started = std::time::Instant::now();
    framework.shutdown_cleanup().await;
    framework.shutdown_cleanup().await; // idempotent
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "shutdown_cleanup exceeded its 5s bound"
    );
}

#[tokio::test]
async fn shutdown_cleanup_is_noop_when_cleanup_disabled() {
    // Default config has cleanup_interval_seconds == 0, so shutdown is a
    // fast no-op.
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let started = std::time::Instant::now();
    framework.shutdown_cleanup().await;
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}
