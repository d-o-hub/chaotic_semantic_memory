use chaotic_semantic_memory::prelude::*;
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
