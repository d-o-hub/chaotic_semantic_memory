use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn import_json_rejects_corrupt_data() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    tokio::fs::write(&path, b"not valid json {{{")
        .await
        .unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.import_json(&path, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_binary_rejects_corrupt_data() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    tokio::fs::write(&path, &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01])
        .await
        .unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.import_binary(&path, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn import_json_empty_payload_succeeds() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let payload = serde_json::json!({
        "version": "test",
        "exported_at": 0u64,
        "concepts": [],
        "associations": []
    });
    tokio::fs::write(&path, serde_json::to_vec(&payload).unwrap())
        .await
        .unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let imported = framework.import_json(&path, false).await.unwrap();
    assert_eq!(imported, 0);
    assert_eq!(framework.stats().await.unwrap().concept_count, 0);
}

#[tokio::test]
async fn import_binary_rejects_oversized_payload() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    // Write >100MB of data to trigger the size guard
    let big = vec![0xABu8; 101 * 1024 * 1024];
    tokio::fs::write(&path, &big).await.unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let result = framework.import_binary(&path, false).await;
    assert!(result.is_err());
}
