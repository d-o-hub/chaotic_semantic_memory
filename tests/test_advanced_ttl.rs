use chaotic_semantic_memory::ChaoticSemanticFramework;
use chaotic_semantic_memory::framework_ttl_advanced::{DecayCurve, TtlConfig, TtlPolicy, TtlRule};
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
    let rules = vec![TtlRule {
        key: "type".to_string(),
        value: serde_json::json!("temp"),
        ttl_seconds: 1,
    }];
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
    fw.inject_concept_with_metadata("c1", HVec10240::random(), metadata)
        .await
        .unwrap();

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
    assert!((assocs[0].1 - 1.0).abs() < f32::EPSILON);
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
    fw.inject_concept_with_ttl("c1", HVec10240::random(), 1)
        .await
        .unwrap();
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
    fw.inject_concept_with_ttl("source", HVec10240::random(), 100)
        .await
        .unwrap();

    // New concept that will associate with source
    fw.inject_concept("child", HVec10240::random())
        .await
        .unwrap();

    // Associate child -> source
    fw.associate("child", "source", 1.0).await.unwrap();

    // Now trigger a re-injection or simulate what evaluate_ttl_policy does.
    fw.inject_concept("child", HVec10240::random())
        .await
        .expect("fw.inject_concept(\"child\") failed");

    let result = fw
        .get_concept("child")
        .await
        .expect("fw.get_concept(\"child\") returned Err");
    let child = result.expect("concept 'child' not found; expected it to exist after injection");
    assert!(child.expires_at.is_some());
    // Verify exact TTL value to kill mutants in evaluate_ttl_policy
    let ttl = child.expires_at.unwrap() - chaotic_semantic_memory::singularity::unix_now_secs();
    assert!(ttl > 90 && ttl <= 100);
}

#[tokio::test]
async fn test_evaluate_ttl_policy_boundary() {
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

    // Inject a concept that expires EXACTLY NOW
    // We use inject_concept_with_ttl which adds ttl to now.
    fw.inject_concept_with_ttl("source", HVec10240::random(), 0)
        .await
        .unwrap();

    fw.inject_concept("child", HVec10240::random())
        .await
        .unwrap();
    fw.associate("child", "source", 1.0).await.unwrap();

    // Inheritance should return None if exp <= now
    fw.inject_concept("child", HVec10240::random())
        .await
        .expect("fw.inject_concept(\"child\") failed");
    let result = fw
        .get_concept("child")
        .await
        .expect("fw.get_concept(\"child\") returned Err");
    let child = result.expect("concept 'child' not found; expected it to exist after injection");
    assert!(child.expires_at.is_none());
}

#[tokio::test]
async fn test_get_associations_returns_correct_data() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("a", HVec10240::random()).await.unwrap();
    fw.inject_concept("b", HVec10240::random()).await.unwrap();
    fw.inject_concept("c", HVec10240::random()).await.unwrap();
    fw.associate("a", "b", 0.8).await.unwrap();
    fw.associate("a", "c", 0.5).await.unwrap();

    let assocs = fw.get_associations("a").await.unwrap();
    assert_eq!(assocs.len(), 2);
    // Verify actual IDs and strengths
    let b_assoc = assocs.iter().find(|(id, _)| id == "b").unwrap();
    assert!((b_assoc.1 - 0.8).abs() < 0.01);
    let c_assoc = assocs.iter().find(|(id, _)| id == "c").unwrap();
    assert!((c_assoc.1 - 0.5).abs() < 0.01);

    // No associations for a concept with none
    let empty = fw.get_associations("b").await.unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn test_incoming_associations_returns_correct_data() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("a", HVec10240::random()).await.unwrap();
    fw.inject_concept("b", HVec10240::random()).await.unwrap();
    fw.inject_concept("c", HVec10240::random()).await.unwrap();
    fw.associate("a", "c", 0.9).await.unwrap();
    fw.associate("b", "c", 0.6).await.unwrap();

    let incoming = fw.incoming_associations("c").await.unwrap();
    assert_eq!(incoming.len(), 2);
    let a_incoming = incoming.iter().find(|(id, _)| id == "a").unwrap();
    assert!((a_incoming.1 - 0.9).abs() < 0.01);
    let b_incoming = incoming.iter().find(|(id, _)| id == "b").unwrap();
    assert!((b_incoming.1 - 0.6).abs() < 0.01);

    // No incoming for a concept with none
    let empty = fw.incoming_associations("a").await.unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn test_purge_expired_returns_count() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Inject concepts that expire immediately
    fw.inject_concept_with_ttl("exp1", HVec10240::random(), 0)
        .await
        .unwrap();
    fw.inject_concept_with_ttl("exp2", HVec10240::random(), 0)
        .await
        .unwrap();
    fw.inject_concept("permanent", HVec10240::random())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let count = fw.purge_expired().await.unwrap();
    assert_eq!(count, 2);

    // Second purge should return 0
    let count2 = fw.purge_expired().await.unwrap();
    assert_eq!(count2, 0);

    // Permanent concept still exists
    assert!(fw.get_concept("permanent").await.unwrap().is_some());
}

#[tokio::test]
async fn test_inject_concept_actually_stores_data() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("stored", HVec10240::random())
        .await
        .unwrap();

    let concept = fw.get_concept("stored").await.unwrap().unwrap();
    assert_eq!(concept.id, "stored");
}

#[tokio::test]
async fn test_inject_concept_with_metadata_stores_metadata() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), serde_json::json!("value"));
    fw.inject_concept_with_metadata("meta-test", HVec10240::random(), metadata)
        .await
        .unwrap();

    let concept = fw.get_concept("meta-test").await.unwrap().unwrap();
    assert_eq!(
        concept.metadata.get("key"),
        Some(&serde_json::json!("value"))
    );
}

#[tokio::test]
async fn test_metadata_rule_no_match_returns_no_ttl() {
    let rules = vec![TtlRule {
        key: "type".to_string(),
        value: serde_json::json!("temp"),
        ttl_seconds: 60,
    }];
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

    // Metadata that does NOT match
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), serde_json::json!("permanent"));
    fw.inject_concept_with_metadata("no-match", HVec10240::random(), metadata)
        .await
        .unwrap();

    let concept = fw.get_concept("no-match").await.unwrap().unwrap();
    assert!(concept.expires_at.is_none());
}

#[tokio::test]
async fn test_metadata_rule_key_missing_returns_no_ttl() {
    let rules = vec![TtlRule {
        key: "type".to_string(),
        value: serde_json::json!("temp"),
        ttl_seconds: 60,
    }];
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

    // No metadata at all
    fw.inject_concept("no-meta", HVec10240::random())
        .await
        .unwrap();

    let concept = fw.get_concept("no-meta").await.unwrap().unwrap();
    assert!(concept.expires_at.is_none());
}

#[tokio::test]
async fn test_ttl_config_builder_integration() {
    let ttl_config = TtlConfig {
        policy: TtlPolicy::Fixed(300),
        association_decay: DecayCurve::Linear { limit_seconds: 60 },
        cleanup_interval_seconds: 0,
        cascading_purge: false,
    };
    let fw = ChaoticSemanticFramework::builder()
        .with_ttl_config(ttl_config.clone())
        .without_persistence()
        .build()
        .await
        .unwrap();

    // Verify the config took effect - inject a concept and check TTL
    fw.inject_concept("cfg-test", HVec10240::random())
        .await
        .unwrap();
    let concept = fw.get_concept("cfg-test").await.unwrap().unwrap();
    assert!(concept.expires_at.is_some());
    let ttl = concept.expires_at.unwrap() - chaotic_semantic_memory::singularity::unix_now_secs();
    assert!(ttl > 290 && ttl <= 300);
}

#[tokio::test]
async fn test_inherit_no_associations_returns_no_ttl() {
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

    // No associations: inherit should yield no TTL
    fw.inject_concept("orphan", HVec10240::random())
        .await
        .unwrap();
    let concept = fw.get_concept("orphan").await.unwrap().unwrap();
    assert!(concept.expires_at.is_none());
}

#[tokio::test]
async fn test_inherit_source_no_ttl_returns_no_ttl() {
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

    // Source has no TTL
    fw.inject_concept("source", HVec10240::random())
        .await
        .unwrap();
    fw.inject_concept("child", HVec10240::random())
        .await
        .unwrap();
    fw.associate("child", "source", 1.0).await.unwrap();

    // Re-inject to trigger evaluate_ttl_policy
    fw.inject_concept("child", HVec10240::random())
        .await
        .unwrap();

    let concept = fw.get_concept("child").await.unwrap().unwrap();
    assert!(concept.expires_at.is_none());
}

#[tokio::test]
async fn test_shortest_path_hops_finds_path() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    fw.inject_concept("a", HVec10240::random()).await.unwrap();
    fw.inject_concept("b", HVec10240::random()).await.unwrap();
    fw.inject_concept("c", HVec10240::random()).await.unwrap();
    fw.associate("a", "b", 1.0).await.unwrap();
    fw.associate("b", "c", 1.0).await.unwrap();

    let path = fw.shortest_path_hops("a", "c").await.unwrap();
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], "a");
    assert_eq!(path[1], "b");
    assert_eq!(path[2], "c");

    // Non-existent path
    fw.inject_concept("isolated", HVec10240::random())
        .await
        .unwrap();
    let no_path = fw.shortest_path_hops("a", "isolated").await.unwrap();
    assert!(no_path.is_none());
}

#[tokio::test]
async fn test_load_without_persistence_is_noop() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    // load() on a non-persistent framework should succeed as a no-op
    fw.load().await.unwrap();
}
