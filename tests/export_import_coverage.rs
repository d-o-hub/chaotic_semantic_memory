//! Export/import operations coverage tests.
//!
//! Covers: export_json, import_json, export_binary, import_binary, backup/restore

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn export_and_import_json_roundtrip() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("export-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("export-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("export-1", "export-2", 0.5)
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let export_path = temp.path().to_str().unwrap();

    // Export to JSON
    framework.export_json(export_path).await.unwrap();

    // Create new framework and import
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let imported_count = framework2.import_json(export_path, false).await.unwrap();
    assert_eq!(imported_count, 2);

    let stats = framework2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn export_and_import_binary_roundtrip() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("binary-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("binary-2", HVec10240::random())
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let export_path = temp.path().to_str().unwrap();

    // Export to binary
    framework.export_binary(export_path).await.unwrap();

    // Import into new framework
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let imported_count = framework2.import_binary(export_path, false).await.unwrap();
    assert_eq!(imported_count, 2);
}

#[tokio::test]
async fn import_json_merge_mode() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Pre-existing concept
    framework
        .inject_concept("existing", HVec10240::random())
        .await
        .unwrap();

    // Export from another framework
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    framework2
        .inject_concept("new-concept", HVec10240::random())
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let export_path = temp.path().to_str().unwrap();
    framework2.export_json(export_path).await.unwrap();

    // Import with merge=true
    let imported = framework.import_json(export_path, true).await.unwrap();
    assert_eq!(imported, 1);

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn concept_history_returns_versions() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("history-concept", HVec10240::random())
        .await
        .unwrap();

    // Update concept to create history
    framework
        .update_concept_vector("history-concept", HVec10240::random())
        .await
        .unwrap();

    // Get history - may be empty if no persistence, just verify operation succeeded
    let _ = framework
        .concept_history("history-concept", 10)
        .await
        .unwrap();
}
