#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Comprehensive integration tests for CSM concept version history (ADR-0074).

use chaotic_semantic_memory::prelude::*;
#[cfg(feature = "persistence")]
use serde_json::json;
#[cfg(feature = "persistence")]
use std::collections::HashMap;
#[cfg(feature = "persistence")]
use tempfile::NamedTempFile;

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_framework_version_history_flow() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    // 1. Build framework with local DB and version retention enabled
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .with_version_retention(10)
        .build()
        .await
        .unwrap();

    let concept_id = "test-version-flow";

    // 2. Inject initial concept (Version 1)
    let v1 = HVec10240::random();
    let mut metadata = HashMap::new();
    metadata.insert("status".to_string(), serde_json::json!("draft"));
    metadata.insert("author".to_string(), serde_json::json!("Alice"));

    framework
        .inject_concept_with_metadata(concept_id, v1, metadata.clone())
        .await
        .unwrap();

    framework.persist().await.unwrap();

    // Verify version 1 exists
    let history = framework.list_versions(concept_id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].version, 1);
    assert!(history[0].vector_changed.unwrap_or(false));
    assert!(history[0].metadata_changed.unwrap_or(false));

    // 3. Update concept vector only (Version 2)
    let v2 = HVec10240::random();
    framework
        .update_concept_vector(concept_id, v2)
        .await
        .unwrap();

    framework.persist().await.unwrap();

    let history = framework.list_versions(concept_id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].version, 2);
    assert!(history[1].vector_changed.unwrap_or(false));
    assert!(!history[1].metadata_changed.unwrap_or(false)); // Metadata unchanged

    // 4. Update concept metadata only (Version 3)
    let mut metadata_v3 = metadata.clone();
    metadata_v3.insert("status".to_string(), serde_json::json!("published"));
    framework
        .update_concept_metadata(concept_id, metadata_v3.clone())
        .await
        .unwrap();

    framework.persist().await.unwrap();

    let history = framework.list_versions(concept_id).await.unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[2].version, 3);
    assert!(!history[2].vector_changed.unwrap_or(false)); // Vector unchanged
    assert!(history[2].metadata_changed.unwrap_or(false));

    // 5. Get specific versions and verify contents
    let c1 = framework.get_version(concept_id, 1).await.unwrap().unwrap();
    assert_eq!(c1.vector, v1);
    assert_eq!(c1.metadata.get("status").unwrap(), "draft");

    let c2 = framework.get_version(concept_id, 2).await.unwrap().unwrap();
    assert_eq!(c2.vector, v2);
    assert_eq!(c2.metadata.get("status").unwrap(), "draft");

    let c3 = framework.get_version(concept_id, 3).await.unwrap().unwrap();
    assert_eq!(c3.vector, v2);
    assert_eq!(c3.metadata.get("status").unwrap(), "published");

    // 6. Diff versions
    let diff_1_to_3 = framework.diff_versions(concept_id, 1, 3).await.unwrap();
    assert!(diff_1_to_3.vector_cosine_distance > 0.0);
    assert_eq!(
        diff_1_to_3.metadata_changed.get("status").unwrap().0,
        serde_json::json!("draft")
    );
    assert_eq!(
        diff_1_to_3.metadata_changed.get("status").unwrap().1,
        serde_json::json!("published")
    );

    // 7. Non-destructive rollback to Version 1 (creates Version 4)
    let rolled = framework.rollback_to_version(concept_id, 1).await.unwrap();
    assert_eq!(rolled.vector, v1);
    assert_eq!(rolled.metadata.get("status").unwrap(), "draft");

    framework.persist().await.unwrap();

    // Verify history now has 4 versions
    let history = framework.list_versions(concept_id).await.unwrap();
    assert_eq!(history.len(), 4);
    assert_eq!(history[3].version, 4);

    let c4 = framework.get_version(concept_id, 4).await.unwrap().unwrap();
    assert_eq!(c4.vector, v1);
    assert_eq!(c4.metadata.get("status").unwrap(), "draft");
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_diff_versions_returns_nontrivial_result() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .with_version_retention(10)
        .build()
        .await
        .unwrap();

    let concept_id = "test-diff-nontrivial";
    let v1 = HVec10240::random();
    let v2 = HVec10240::random();

    let mut meta1 = HashMap::new();
    meta1.insert("status".to_string(), json!("draft"));
    let mut meta2 = HashMap::new();
    meta2.insert("status".to_string(), json!("published"));

    framework
        .inject_concept_with_metadata(concept_id, v1, meta1)
        .await
        .unwrap();
    framework
        .update_concept_vector(concept_id, v2)
        .await
        .unwrap();
    framework
        .update_concept_metadata(concept_id, meta2)
        .await
        .unwrap();

    let diff = framework.diff_versions(concept_id, 1, 3).await.unwrap();
    assert!(
        diff.vector_cosine_distance > 0.0,
        "cosine distance must be positive for different vectors, got {}",
        diff.vector_cosine_distance
    );
    assert!(
        !diff.metadata_changed.is_empty(),
        "metadata_changed must not be empty"
    );
}

#[tokio::test]
async fn test_version_history_unsupported_when_no_persistence() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let concept_id = "test-no-persist";
    framework
        .inject_concept(concept_id, HVec10240::random())
        .await
        .unwrap();

    // Verify that all 4 APIs return UnsupportedOperation when persistence is disabled
    assert!(matches!(
        framework.list_versions(concept_id).await.unwrap_err(),
        MemoryError::UnsupportedOperation(_)
    ));

    assert!(matches!(
        framework.get_version(concept_id, 1).await.unwrap_err(),
        MemoryError::UnsupportedOperation(_)
    ));

    assert!(matches!(
        framework.diff_versions(concept_id, 1, 2).await.unwrap_err(),
        MemoryError::UnsupportedOperation(_)
    ));

    assert!(matches!(
        framework
            .rollback_to_version(concept_id, 1)
            .await
            .unwrap_err(),
        MemoryError::UnsupportedOperation(_)
    ));
}
