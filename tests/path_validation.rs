//! Path validation edge cases for framework_ops.rs coverage.
//!
//! Covers: validate_path branches, import error handling, backup/restore

use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn export_json_path_too_long_rejected() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("long-path", HVec10240::random())
        .await
        .unwrap();

    // Path exceeding 4096 characters
    let long_path = "/tmp/".to_string() + &"x".repeat(5000);
    let result = framework.export_json(&long_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn export_json_path_traversal_rejected() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("traversal", HVec10240::random())
        .await
        .unwrap();

    // Path with parent directory traversal
    let traversal_path = "/tmp/../etc/passwd";
    let result = framework.export_json(traversal_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_json_skips_invalid_association() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Construct a JSON payload with an invalid association (negative strength).
    // HVec10240 serializes as a JSON byte array (via serialize_bytes).
    let vector1 = HVec10240::random();
    let vector2 = HVec10240::random();
    let vector3 = HVec10240::random();

    // Get the JSON representation of each HVec10240 (array of 40960 u8 values)
    let v1_json = serde_json::to_value(&vector1).unwrap();
    let v2_json = serde_json::to_value(&vector2).unwrap();
    let v3_json = serde_json::to_value(&vector3).unwrap();

    let payload_json = serde_json::json!({
        "version": "0.3.5",
        "exported_at": 12345,
        "concepts": [
            {
                "id": "import-invalid-0",
                "vector": v1_json,
                "metadata": {},
                "created_at": 1,
                "modified_at": 1
            },
            {
                "id": "import-invalid-1",
                "vector": v2_json,
                "metadata": {},
                "created_at": 1,
                "modified_at": 1
            },
            {
                "id": "import-invalid-2",
                "vector": v3_json,
                "metadata": {},
                "created_at": 1,
                "modified_at": 1
            }
        ],
        "associations": [
            ["import-invalid-0", "import-invalid-1", 0.5],
            ["import-invalid-0", "import-invalid-2", -0.5]
        ]
    });

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    std::fs::write(path, serde_json::to_string(&payload_json).unwrap()).unwrap();

    // Import should succeed, skipping the invalid association
    let imported = framework.import_json(path, false).await.unwrap();
    assert_eq!(imported, 3);
}

#[tokio::test]
async fn backup_and_restore_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .with_local_db(db_path.clone())
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("backup-1", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("backup-2", HVec10240::random())
        .await
        .unwrap();
    framework.persist().await.unwrap();

    // Backup
    let backup_temp = NamedTempFile::new().unwrap();
    let backup_path = backup_temp.path().to_str().unwrap();
    framework.backup(backup_path).await.unwrap();

    // Restore into new framework
    let restore_temp = NamedTempFile::new().unwrap();
    let restore_path = restore_temp.path().to_str().unwrap().to_string();
    let framework2: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .with_local_db(restore_path)
        .build()
        .await
        .unwrap();

    framework2.restore(backup_path).await.unwrap();

    let stats = framework2.stats().await.unwrap();
    assert_eq!(stats.concept_count, 2);
}

#[tokio::test]
async fn concept_history_with_limit_clamping() {
    let temp = NamedTempFile::new().unwrap();
    let db_path = temp.path().to_str().unwrap().to_string();

    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("history-limit", HVec10240::random())
        .await
        .unwrap();

    // Request history with limit > MAX_HISTORY_LIMIT (1000)
    let history = framework
        .concept_history("history-limit", 5000)
        .await
        .unwrap();
    // Should clamp to 1000
    assert!(history.len() <= 1000);
}
