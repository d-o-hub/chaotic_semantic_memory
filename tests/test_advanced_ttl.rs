use chaotic_semantic_memory::ChaoticSemanticFramework;
use chaotic_semantic_memory::framework_ttl_advanced::{TtlConfig, TtlPolicy, TtlRule, DecayCurve};
use csm_core::hyperdim::HVec10240;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_fixed_ttl_policy() {
    let ttl_config = TtlConfig {
        policy: TtlPolicy::Fixed(1), // 1 second TTL
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config)
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("c1", HVec10240::random()).await.unwrap();

    let concept = fw.get_concept("c1").await.unwrap().unwrap();
    assert!(concept.expires_at.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    let results = fw.probe(HVec10240::random(), 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_metadata_ttl_policy() {
    let rules = vec![
        TtlRule {
            key: "type".to_string(),
            value: serde_json::json!("temp"),
            ttl_seconds: 1,
        }
    ];
    let ttl_config = TtlConfig {
        policy: TtlPolicy::MetadataRule(rules),
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config)
        .without_persistence()
        .build()
        .await
        .unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), serde_json::json!("temp"));
    fw.inject_concept_with_metadata("c1", HVec10240::random(), metadata).await.unwrap();

    let concept = fw.get_concept("c1").await.unwrap().unwrap();
    assert!(concept.expires_at.is_some());

    fw.inject_concept("c2", HVec10240::random()).await.unwrap();
    let concept2 = fw.get_concept("c2").await.unwrap().unwrap();
    assert!(concept2.expires_at.is_none());
}

#[tokio::test]
async fn test_linear_decay() {
    let ttl_config = TtlConfig {
        association_decay: DecayCurve::Linear { limit_seconds: 10 },
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config)
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("c1", HVec10240::random()).await.unwrap();
    fw.inject_concept("c2", HVec10240::random()).await.unwrap();
    fw.associate("c1", "c2", 1.0).await.unwrap();

    let assocs = fw.get_associations("c1").await.unwrap();
    assert_eq!(assocs[0].1, 1.0);
}

#[tokio::test]
async fn test_cascading_purge() {
    let ttl_config = TtlConfig {
        cascading_purge: true,
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config)
        .without_persistence()
        .build()
        .await
        .unwrap();

    // c1 expires soon, c2 is permanent but depends on c1
    fw.inject_concept_with_ttl("c1", HVec10240::random(), 1).await.unwrap();
    fw.inject_concept("c2", HVec10240::random()).await.unwrap();
    fw.associate("c1", "c2", 1.0).await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;
    fw.purge_expired().await.unwrap();

    assert!(fw.get_concept("c1").await.unwrap().is_none());
    assert!(fw.get_concept("c2").await.unwrap().is_none()); // Purged via cascade
}

#[tokio::test]
async fn test_inherit_ttl_policy() {
    let ttl_config = TtlConfig {
        policy: TtlPolicy::Inherit,
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config)
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Source concept with TTL
    fw.inject_concept_with_ttl("source", HVec10240::random(), 100).await.unwrap();

    // New concept that will associate with source
    fw.inject_concept("child", HVec10240::random()).await.unwrap();

    // Associate child -> source
    fw.associate("child", "source", 1.0).await.unwrap();

    // Now trigger a re-injection or simulate what evaluate_ttl_policy does.
    fw.inject_concept("child", HVec10240::random()).await.unwrap();

    let child = fw.get_concept("child").await.unwrap().unwrap();
    assert!(child.expires_at.is_some());
}
