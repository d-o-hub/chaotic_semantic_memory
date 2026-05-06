use chaotic_semantic_memory::MemoryError;
use chaotic_semantic_memory::prelude::*;
use std::sync::Arc;
use tempfile::NamedTempFile;

const NS: &str = "_default";

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

    framework.delete_concept("b").await.unwrap();
    // Reload framework2 from persistence to see the deletion
    framework2.load_replace().await.unwrap();
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

#[tokio::test]
async fn binary_import_export_preserves_ttl_and_canonical_links() {
    let temp_db = NamedTempFile::new().unwrap();
    let db_path = temp_db.path().to_str().unwrap();
    let import_file = NamedTempFile::new().unwrap();
    let import_path = import_file.path().to_str().unwrap().to_string();
    let export_file = NamedTempFile::new().unwrap();
    let export_path = export_file.path().to_str().unwrap().to_string();

    let payload = serde_json::json!({
        "version": "0.3.2",
        "exported_at": 1u64,
        "concepts": [{
            "id": "ttl-canonical",
            "vector": HVec10240::random(),
            "metadata": {"kind": "regression"},
            "created_at": 10u64,
            "modified_at": 11u64,
            "expires_at": 777u64,
            "canonical_concept_ids": ["concept.alpha", "concept.beta"]
        }],
        "associations": []
    });
    tokio::fs::write(&import_path, serde_json::to_vec(&payload).unwrap())
        .await
        .unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    framework.import_json(&import_path, false).await.unwrap();
    framework.export_binary(&export_path).await.unwrap();

    let reload = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    reload.import_binary(&export_path, false).await.unwrap();

    let concept = reload.get_concept("ttl-canonical").await.unwrap().unwrap();
    assert_eq!(concept.expires_at, Some(777));
    assert_eq!(
        concept.canonical_concept_ids,
        vec!["concept.alpha".to_string(), "concept.beta".to_string()]
    );
}

#[tokio::test]
async fn load_merge_does_not_silently_overwrite_existing_concepts() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework_a = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .build()
        .await
        .unwrap();
    let framework_b = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let vector_a = HVec10240::random();
    let vector_b = HVec10240::random();

    framework_a
        .inject_concept("shared-id", vector_a)
        .await
        .unwrap();

    // Persist a different value through another instance.
    framework_b
        .inject_concept("shared-id", vector_b)
        .await
        .unwrap();

    framework_a.load_merge().await.unwrap();

    let concept = framework_a.get_concept("shared-id").await.unwrap().unwrap();
    assert_eq!(concept.vector, vector_a);
}
