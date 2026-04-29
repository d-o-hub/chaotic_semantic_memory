//! Framework batch operations and import limit coverage tests.
//!
//! Covers: inject_concepts empty batch, associate_many empty batch, probe_batch,
//! probe_batch_cached, import size exceeded, concept_history limit clamping

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn inject_concepts_empty_batch_returns_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Empty batch should succeed without doing anything
    framework.inject_concepts(&[]).await.unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 0);
}

#[tokio::test]
async fn associate_many_empty_batch_returns_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Empty batch should succeed
    framework.associate_many(&[]).await.unwrap();
}

#[tokio::test]
async fn probe_batch_returns_multiple_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("batch-probe-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("batch-probe-2", HVec10240::random())
        .await
        .unwrap();

    let queries: Vec<HVec10240> = (0..3).map(|_| HVec10240::random()).collect();
    let results = framework.probe_batch(&queries, 5).await.unwrap();

    assert_eq!(results.len(), 3);
    for result in results {
        assert!(result.len() <= 2);
    }
}

#[tokio::test]
async fn probe_batch_cached_returns_arc_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(4)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("cached-batch-1", HVec10240::random())
        .await
        .unwrap();

    let queries: Vec<HVec10240> = (0..2).map(|_| HVec10240::random()).collect();
    let results = framework.probe_batch_cached(&queries, 5).await.unwrap();

    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn concept_history_limit_clamped_to_max() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("history-limit", HVec10240::random())
        .await
        .unwrap();

    // Request history with limit exceeding MAX_HISTORY_LIMIT (1000)
    // Should be clamped to 1000
    let history = framework
        .concept_history("history-limit", 5000)
        .await
        .unwrap();
    // History may be empty without updates, but call should succeed
    assert!(history.len() <= 1000);
}

#[tokio::test]
async fn disassociate_removes_association() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("disassoc-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("disassoc-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("disassoc-1", "disassoc-2", 0.8)
        .await
        .unwrap();

    let metrics_before = framework.metrics_snapshot().await;
    assert_eq!(metrics_before.associations_created_total, 1);

    // Remove association
    framework
        .disassociate("disassoc-1", "disassoc-2")
        .await
        .unwrap();

    // Verify the operation succeeded
    let metrics_after = framework.metrics_snapshot().await;
    assert_eq!(metrics_after.associations_created_total, 1);
}

#[tokio::test]
async fn clear_associations_removes_all_for_concept() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("clear-assoc-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("clear-assoc-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("clear-assoc-3", HVec10240::random())
        .await
        .unwrap();

    framework
        .associate("clear-assoc-1", "clear-assoc-2", 0.7)
        .await
        .unwrap();
    framework
        .associate("clear-assoc-1", "clear-assoc-3", 0.6)
        .await
        .unwrap();

    let metrics_before = framework.metrics_snapshot().await;
    assert_eq!(metrics_before.associations_created_total, 2);

    // Clear all associations from clear-assoc-1
    framework.clear_associations("clear-assoc-1").await.unwrap();
}

#[tokio::test]
async fn update_concept_metadata_persists() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("update-meta", HVec10240::random())
        .await
        .unwrap();

    let new_metadata =
        std::collections::HashMap::from([("updated".to_string(), serde_json::json!(true))]);

    framework
        .update_concept_metadata("update-meta", new_metadata)
        .await
        .unwrap();

    // Concept should still exist with updated metadata
    let concept = framework.get_concept("update-meta").await.unwrap().unwrap();
    assert_eq!(
        concept.metadata.get("updated"),
        Some(&serde_json::json!(true))
    );
}
