#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Lib-target guards for two boundaries the mutation gate could not kill on
//! `feat/audit-followups-wave-2026-10-22` (job 123043631991): `delete !` in
//! `ChaoticSemanticFramework::reinforce_association` (src/framework.rs:412) and
//! `replace < with <=` in `ChaoticSemanticFramework::prune_decayed_associations`
//! (src/framework.rs:460). Register in src/lib.rs next to `mod framework_ops_tests;`.

use crate::framework::ChaoticSemanticFramework;
use crate::framework_ttl_advanced::{DecayCurve, TtlConfig};
use csm_core_lib::error::MemoryError;
use csm_core_lib::hyperdim::HVec10240;

#[tokio::test]
async fn reinforce_association_splits_on_existence() {
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("live".to_string(), HVec10240::random()),
        ("other".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate("live", "other", 0.7).await.unwrap();

    fw.reinforce_association("live", "other")
        .await
        .expect("a real association must reinforce");

    let err = fw
        .reinforce_association("live", "ghost")
        .await
        .expect_err("a never-created association must not reinforce");
    let MemoryError::NotFound { entity, id } = err else {
        panic!("expected NotFound, got: {err:?}");
    };
    assert_eq!(entity, "Association");
    assert_eq!(id, "live -> ghost");
}

#[tokio::test]
async fn prune_decayed_associations_threshold_is_exclusive() {
    let curve = DecayCurve::Step {
        threshold_seconds: 0,
        drop: 0.4,
    };
    let ttl = TtlConfig {
        association_decay: curve,
        ..Default::default()
    };
    let fw = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_ttl_config(ttl)
        .build()
        .await
        .unwrap();
    fw.inject_concepts(&[
        ("edge-a".to_string(), HVec10240::random()),
        ("edge-b".to_string(), HVec10240::random()),
    ])
    .await
    .unwrap();
    fw.associate("edge-a", "edge-b", 0.9).await.unwrap();

    // Derived from the same curve the framework applies, so the comparison lands exactly
    // on the boundary instead of on a hand-written float that may not.
    let decayed = curve.apply(0.9, 0);

    assert_eq!(
        fw.prune_decayed_associations(decayed).await.unwrap(),
        0,
        "an association whose decayed strength equals the threshold must survive"
    );

    assert_eq!(
        fw.prune_decayed_associations(decayed + 0.1).await.unwrap(),
        1,
        "one step above the threshold must prune it"
    );
}
