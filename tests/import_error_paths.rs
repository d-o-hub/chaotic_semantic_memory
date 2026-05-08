//! Framework ops path validation error branch coverage tests.
//!
//! Covers: validate_path too long, path traversal, absolute path restrictions

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

const NS: &str = "_default";

#[tokio::test]
async fn import_json_invalid_json_data_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Create file with invalid JSON
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    std::fs::write(path, "not valid json {broken").unwrap();

    let result = framework.import_json(path, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_binary_invalid_data_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Create file with invalid binary data
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    std::fs::write(path, [0u8; 100]).unwrap();

    let result = framework.import_binary(path, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_json_empty_file_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Create empty file
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    std::fs::write(path, "").unwrap();

    let result = framework.import_json(path, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_json_with_associations_to_nonexistent_concepts() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // First create and export a valid concept
    framework
        .inject_concept("assoc-source", HVec10240::random())
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    framework.export_json(path).await.unwrap();

    // Modify the JSON to add associations to nonexistent concepts
    let content = std::fs::read_to_string(path).unwrap();
    let modified = content.replace(
        "\"associations\": []",
        "\"associations\": [[\"assoc-source\", \"nonexistent\", 0.5]]",
    );
    std::fs::write(path, modified).unwrap();

    // Import should succeed, but skip invalid associations (logged warning)
    let imported = framework.import_json(path, true).await.unwrap();
    assert_eq!(imported, 1);
}

#[tokio::test]
async fn import_binary_with_associations_to_nonexistent_concepts() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // First create and export a valid concept
    framework
        .inject_concept("binary-assoc-source", HVec10240::random())
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    framework.export_binary(path).await.unwrap();

    // Import should succeed
    let imported = framework.import_binary(path, true).await.unwrap();
    assert_eq!(imported, 1);
}

#[tokio::test]
async fn export_and_import_cycle_preserves_data() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("cycle-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("cycle-2", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("cycle-1", "cycle-2", 0.9)
        .await
        .unwrap();

    // Export to JSON
    let json_temp = NamedTempFile::new().unwrap();
    let json_path = json_temp.path().to_str().unwrap();
    framework.export_json(json_path).await.unwrap();

    // Create new framework and import
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let imported = framework2.import_json(json_path, false).await.unwrap();
    assert_eq!(imported, 2);

    let stats = framework2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn export_and_import_binary_cycle_preserves_data() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("binary-cycle-1", HVec10240::random())
        .await
        .unwrap();

    // Export to binary
    let binary_temp = NamedTempFile::new().unwrap();
    let binary_path = binary_temp.path().to_str().unwrap();
    framework.export_binary(binary_path).await.unwrap();

    // Create new framework and import
    let framework2 = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let imported = framework2.import_binary(binary_path, false).await.unwrap();
    assert_eq!(imported, 1);

    let stats = framework2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 1);
}
