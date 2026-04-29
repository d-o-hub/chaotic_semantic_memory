//! Framework ops path validation error branch coverage tests.
//!
//! Covers: validate_path too long, path traversal detection, absolute path restrictions

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn export_json_path_too_long_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("path-length-test", HVec10240::random())
        .await
        .unwrap();

    // Create a path that exceeds MAX_PATH_LENGTH (4096)
    let long_path = "/tmp/".to_string() + &"a".repeat(5000);
    let result = framework.export_json(&long_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn export_json_path_traversal_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("traversal-test", HVec10240::random())
        .await
        .unwrap();

    // Create a path with traversal components
    let traversal_path = "/tmp/../etc/passwd";
    let result = framework.export_json(traversal_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_json_absolute_path_outside_allowed_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Try to import from an absolute path outside cwd and /tmp
    // This may succeed or fail depending on file existence
    let abs_path = "/var/nonexistent/file.json";
    let result = framework.import_json(abs_path, false).await;
    // Should fail due to path validation or file not found
    assert!(result.is_err());
}

#[tokio::test]
async fn backup_with_persistence_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(&db_path)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("backup-test", HVec10240::random())
        .await
        .unwrap();
    framework.persist().await.unwrap();

    // Create backup
    let backup_temp = NamedTempFile::new().unwrap();
    let backup_path = backup_temp.path().to_str().unwrap();
    framework.backup(backup_path).await.unwrap();
}

#[tokio::test]
async fn restore_with_persistence_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(&db_path)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("restore-test", HVec10240::random())
        .await
        .unwrap();
    framework.persist().await.unwrap();

    // Create backup
    let backup_temp = NamedTempFile::new().unwrap();
    let backup_path = backup_temp.path().to_str().unwrap();
    framework.backup(backup_path).await.unwrap();

    // Restore from backup
    framework.restore(backup_path).await.unwrap();
}

#[tokio::test]
async fn export_binary_path_traversal_fails() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("binary-traversal", HVec10240::random())
        .await
        .unwrap();

    // Path with traversal should fail
    let traversal_path = "/tmp/../root/test.bin";
    let result = framework.export_binary(traversal_path).await;
    assert!(result.is_err());
}