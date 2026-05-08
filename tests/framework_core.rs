use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240, Hypervector};

const NS: &str = "_default";

#[tokio::test]
async fn framework_creation_starts_empty() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let stats = framework.stats().await.unwrap();
    assert_eq!(stats.concept_count, 0);
}

#[tokio::test]
async fn concept_lifecycle_updates_associations() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("a", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();
    framework.associate("a", "b", 0.8).await.unwrap();

    let associations = framework.get_associations("a").await.unwrap();
    assert_eq!(associations.len(), 1);

    framework.delete_concept("b").await.unwrap();
    let associations = framework.get_associations("a").await.unwrap();
    assert!(associations.is_empty());
}
