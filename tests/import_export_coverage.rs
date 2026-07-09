#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Absolute path validation coverage for framework_ops.rs.
//!
//! Covers: validate_path absolute path branches, backup without persistence

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn export_json_absolute_path_in_current_dir() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("abs-path-1", HVec10240::random())
        .await
        .unwrap();

    // Use absolute path in /tmp (allowed)
    let temp = NamedTempFile::new().unwrap();
    let abs_path = temp.path().to_str().unwrap();

    // This should succeed since /tmp is allowed
    let result = framework.export_json(abs_path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn backup_without_persistence_is_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("no-persist-backup", HVec10240::random())
        .await
        .unwrap();

    // Backup without persistence should succeed (no-op)
    let temp = NamedTempFile::new().unwrap();
    let backup_path = temp.path().to_str().unwrap();
    framework.backup(backup_path).await.unwrap();
}

#[tokio::test]
async fn restore_without_persistence_is_ok() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Restore without persistence should succeed (no-op)
    let temp = NamedTempFile::new().unwrap();
    let restore_path = temp.path().to_str().unwrap();
    framework.restore(restore_path).await.unwrap();
}

#[tokio::test]
async fn concept_history_without_persistence_returns_empty() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("no-persist-history", HVec10240::random())
        .await
        .unwrap();

    // History without persistence returns empty
    let history = framework
        .concept_history("no-persist-history", 10)
        .await
        .unwrap();
    assert!(history.is_empty());
}

#[tokio::test]
async fn import_json_merge_with_existing_concepts() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    // Pre-existing concepts
    framework
        .inject_concept("merge-existing", HVec10240::random())
        .await
        .unwrap();
    framework.persist().await.unwrap();

    // Create another framework and export from it
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    framework2
        .inject_concept("merge-new", HVec10240::random())
        .await
        .unwrap();

    let export_temp = NamedTempFile::new().unwrap();
    let export_path = export_temp.path().to_str().unwrap();
    framework2.export_json(export_path).await.unwrap();

    // Import with merge=true
    let imported = framework.import_json(export_path, true).await.unwrap();
    assert_eq!(imported, 1);

    // Both concepts should exist
    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn import_binary_merge_with_existing_concepts() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    // Pre-existing concepts
    framework
        .inject_concept("binary-merge-existing", HVec10240::random())
        .await
        .unwrap();
    framework.persist().await.unwrap();

    // Create another framework and export from it
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    framework2
        .inject_concept("binary-merge-new", HVec10240::random())
        .await
        .unwrap();

    let export_temp = NamedTempFile::new().unwrap();
    let export_path = export_temp.path().to_str().unwrap();
    framework2.export_binary(export_path).await.unwrap();

    // Import with merge=true
    let imported = framework.import_binary(export_path, true).await.unwrap();
    assert_eq!(imported, 1);

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}
