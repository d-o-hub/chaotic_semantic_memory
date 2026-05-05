use chaotic_semantic_memory::prelude::*;
use tempfile::tempdir;

#[tokio::test]
async fn test_namespace_isolation() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let db_path_str = db_path.to_str().unwrap();

    // 1. Setup two frameworks with different namespaces
    let fw1 = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .with_namespace("ns1")
        .build()
        .await
        .unwrap();

    let fw2 = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .with_namespace("ns2")
        .build()
        .await
        .unwrap();

    // 2. Inject concepts with same ID into different namespaces
    let vec1 = HVec10240::random();
    let vec2 = HVec10240::random();

    fw1.inject_concept("shared-id", vec1).await.unwrap();
    fw2.inject_concept("shared-id", vec2).await.unwrap();

    // 3. Verify isolation in memory
    let c1 = fw1.get_concept("shared-id").await.unwrap().unwrap();
    let c2 = fw2.get_concept("shared-id").await.unwrap().unwrap();

    assert_eq!(c1.vector, vec1);
    assert_eq!(c2.vector, vec2);
    assert_ne!(vec1, vec2);

    // 4. Verify isolation in persistence (fresh load)
    let fw1_reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .with_namespace("ns1")
        .build()
        .await
        .unwrap();

    let fw2_reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .with_namespace("ns2")
        .build()
        .await
        .unwrap();

    let c1_reloaded = fw1_reloaded.get_concept("shared-id").await.unwrap().unwrap();
    let c2_reloaded = fw2_reloaded.get_concept("shared-id").await.unwrap().unwrap();

    assert_eq!(c1_reloaded.vector, vec1);
    assert_eq!(c2_reloaded.vector, vec2);
}

#[tokio::test]
async fn test_default_namespace_compatibility() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("compat.db");
    let db_path_str = db_path.to_str().unwrap();

    // Injected with default
    let fw_default = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .build() // Uses _default
        .await
        .unwrap();

    let vec = HVec10240::random();
    fw_default.inject_concept("c1", vec).await.unwrap();

    // Reload with explicit _default
    let fw_explicit = ChaoticSemanticFramework::builder()
        .with_local_db(db_path_str)
        .with_namespace("_default")
        .build()
        .await
        .unwrap();

    let c1 = fw_explicit.get_concept("c1").await.unwrap().unwrap();
    assert_eq!(c1.vector, vec);
}
