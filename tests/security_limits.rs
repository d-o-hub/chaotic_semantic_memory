use chaotic_semantic_memory::prelude::*;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_builder_clamping_limits() {
    let framework = ChaoticSemanticFramework::builder()
        .with_max_probe_top_k(1_000_000)
        .with_max_batch_size(1_000_000)
        .with_max_sequence_length(1_000_000)
        .with_max_metadata_bytes(1_000_000_000)
        .with_concept_cache_size(1_000_000)
        .with_version_retention(1_000_000)
        .build()
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    // Verify that the framework built despite extreme values (clamping worked)
    assert!(stats.concept_count == 0);
}

#[tokio::test]
async fn test_import_oversized_file() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    // Create a file larger than MAX_IMPORT_SIZE (100MB)
    let f = std::fs::File::create(path).unwrap();
    f.set_len(101 * 1024 * 1024).unwrap();

    let result = framework.import_json(path, false).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(format!("{e:?}").contains("exceeds maximum allowed size"));
    }
}
