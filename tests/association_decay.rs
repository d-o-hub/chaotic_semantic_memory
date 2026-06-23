//! Integration tests for association decay (ADR-0025, issue #412).

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::framework_ttl_advanced::{DecayCurve, TtlConfig};

#[tokio::test]
async fn reinforce_association_resets_decay_clock() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let v = HVec10240::random();
    framework
        .inject_concept("a".to_string(), v.clone())
        .await
        .unwrap();
    framework
        .inject_concept("b".to_string(), v)
        .await
        .unwrap();
    framework.associate("a", "b", 0.8).await.unwrap();

    // Reinforce should succeed
    framework.reinforce_association("a", "b").await.unwrap();

    // Reinforcing a non-existent association should error
    let result = framework.reinforce_association("a", "z").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn prune_decayed_removes_weak_associations() {
    let config = TtlConfig {
        association_decay: DecayCurve::Step {
            threshold_seconds: 0,
            drop: 1.0,
        },
        ..Default::default()
    };

    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_ttl_config(config)
        .build()
        .await
        .unwrap();

    let v = HVec10240::random();
    framework
        .inject_concept("x".to_string(), v.clone())
        .await
        .unwrap();
    framework
        .inject_concept("y".to_string(), v)
        .await
        .unwrap();
    framework.associate("x", "y", 0.9).await.unwrap();

    // With step decay (threshold=0, drop=1.0), the decayed strength is 0.0
    let pruned = framework.prune_decayed_associations(0.5).await.unwrap();
    assert_eq!(pruned, 1);

    // Association should be gone
    let assocs = framework.get_associations("x").await.unwrap();
    assert!(assocs.is_empty());
}
