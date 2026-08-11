#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Framework builder configuration coverage tests.
//!
//! Covers: with_max_sequence_length, with_max_metadata_bytes, with_version_retention

use chaotic_semantic_memory::prelude::*;
#[cfg(feature = "persistence")]
use tempfile::NamedTempFile;

#[tokio::test]
async fn builder_with_max_sequence_length_configured() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_sequence_length(100)
        .build()
        .await
        .unwrap();

    // Verify framework was created with custom config
    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 0);
}

#[tokio::test]
async fn builder_with_max_metadata_bytes_configured() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_metadata_bytes(1024)
        .build()
        .await
        .unwrap();

    // Small metadata should work
    let small_meta =
        std::collections::HashMap::from([("key".to_string(), serde_json::json!("value"))]);
    framework
        .inject_concept_with_metadata("meta-small", HVec10240::random(), small_meta)
        .await
        .unwrap();
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn builder_with_version_retention_configured() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .with_version_retention(3)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("version-retention", HVec10240::random())
        .await
        .unwrap();

    // Multiple updates to create versions
    for _ in 0..5 {
        framework
            .update_concept_vector("version-retention", HVec10240::random())
            .await
            .unwrap();
    }

    // History should be limited by retention
    let history = framework
        .concept_history("version-retention", 10)
        .await
        .unwrap();
    assert!(history.len() <= 3);
}

#[tokio::test]
async fn builder_with_max_batch_size_configured() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_batch_size(5)
        .build()
        .await
        .unwrap();

    // Small batch should work
    let concepts: Vec<(String, HVec10240)> = (0..3)
        .map(|i| (format!("batch-{i}"), HVec10240::random()))
        .collect();
    framework.inject_concepts(&concepts).await.unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 3);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn builder_with_connection_pool_size_configured() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .with_connection_pool_size(5)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("pool-test", HVec10240::random())
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 1);
}

#[tokio::test]
async fn builder_with_chaos_strength_configured() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_chaos_strength(0.5)
        .with_reservoir_size(1000)
        .build()
        .await
        .unwrap();

    // Framework should be created with custom reservoir config
    framework
        .inject_concept("chaos-test", HVec10240::random())
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 1);
}
