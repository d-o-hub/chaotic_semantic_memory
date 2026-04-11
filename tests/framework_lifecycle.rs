use chaotic_semantic_memory::MemoryError;
use chaotic_semantic_memory::prelude::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[tokio::test]
async fn framework_lifecycle_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .with_max_concepts(10)
        .with_max_associations_per_concept(2)
        .build()
        .await
        .unwrap();

    let vec_a = HVec10240::random();
    framework.inject_concept("a", vec_a).await.unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();
    framework.associate("a", "b", 0.7).await.unwrap();

    let probe = framework.probe(vec_a, 2).await.unwrap();
    assert!(!probe.is_empty());

    framework.persist().await.unwrap();

    let framework2 = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let stats = framework2.stats().await.unwrap();
    assert!(stats.concept_count >= 2);

    framework2.delete_concept("b").await.unwrap();
    let links = framework2.get_associations("a").await.unwrap();
    assert!(links.is_empty());
}

#[tokio::test]
async fn framework_input_validation_rejects_invalid_public_inputs() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let empty_id = framework.inject_concept("", HVec10240::random()).await;
    assert!(matches!(empty_id, Err(MemoryError::InvalidInput { .. })));

    let bad_strength = framework.associate("a", "b", f32::NAN).await;
    assert!(matches!(
        bad_strength,
        Err(MemoryError::InvalidInput { .. })
    ));

    let bad_top_k = framework.probe(HVec10240::random(), 0).await;
    assert!(matches!(bad_top_k, Err(MemoryError::InvalidInput { .. })));
}

#[tokio::test]
async fn framework_import_skips_orphan_associations_without_failing() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let import_file = NamedTempFile::new().unwrap();
    let import_path = import_file.path().to_str().unwrap().to_string();

    let payload = serde_json::json!({
        "version": "0.3.2",
        "exported_at": 1u64,
        "concepts": [],
        "associations": [["a", "missing", 0.7]]
    });
    tokio::fs::write(&import_path, serde_json::to_vec(&payload).unwrap())
        .await
        .unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let imported = framework.import_json(&import_path, false).await.unwrap();
    assert_eq!(imported, 0);
    assert_eq!(framework.stats().await.unwrap().concept_count, 0);
}

#[tokio::test]
async fn concurrent_access_with_persistence() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = Arc::new(
        ChaoticSemanticFramework::builder()
            .with_local_db(path)
            .build()
            .await
            .unwrap(),
    );

    let mut jobs = Vec::new();
    for i in 0..8 {
        let framework = Arc::clone(&framework);
        jobs.push(tokio::spawn(async move {
            let id = format!("c{i}");
            framework
                .inject_concept(id.clone(), HVec10240::random())
                .await?;
            framework.probe(HVec10240::random(), 5).await?;
            framework.get_associations(&id).await?;
            Ok::<(), MemoryError>(())
        }));
    }

    for job in jobs {
        job.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn probe_batch_cached_reuses_cached_results() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_concept_cache_size(8)
        .build()
        .await
        .unwrap();

    let vec_a = HVec10240::random();
    framework.inject_concept("a", vec_a).await.unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();

    let queries = [vec_a];
    let first = framework.probe_batch_cached(&queries, 2).await.unwrap();
    let second = framework.probe_batch_cached(&queries, 2).await.unwrap();
    assert!(Arc::ptr_eq(&first[0], &second[0]));
}
