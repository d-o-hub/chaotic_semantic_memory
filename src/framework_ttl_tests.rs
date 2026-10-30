#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit-target tests for the TTL policy evaluator (`src/framework_ttl.rs`).
//!
//! These exist because the evaluator's branches were mutation-invisible: with no
//! `--lib` assertions, `evaluate_ttl_policy` survived as `None`, as `Some(1)`, and
//! with its `exp > now` / `exp - now` arithmetic rewritten, while the whole
//! workspace still compiled and tested green.

use std::collections::HashMap;

use crate::framework::ChaoticSemanticFramework;
use crate::framework_ttl_advanced::{TtlConfig, TtlPolicy, TtlRule};
use csm_core_lib::hyperdim::HVec10240;
use serde_json::json;

async fn framework_with_policy(policy: TtlPolicy) -> ChaoticSemanticFramework {
    ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_ttl_config(TtlConfig {
            policy,
            ..TtlConfig::default()
        })
        .build()
        .await
        .unwrap()
}

#[tokio::test]
async fn none_policy_assigns_no_expiry() {
    let framework = framework_with_policy(TtlPolicy::None).await;
    assert_eq!(
        framework
            .evaluate_ttl_policy("anything", &HashMap::new())
            .await,
        None,
        "TtlPolicy::None must never assign an expiry"
    );
}

#[tokio::test]
async fn fixed_policy_returns_the_configured_seconds() {
    let framework = framework_with_policy(TtlPolicy::Fixed(3600)).await;
    assert_eq!(
        framework
            .evaluate_ttl_policy("anything", &HashMap::new())
            .await,
        Some(3600),
        "TtlPolicy::Fixed must report its own interval, not a constant"
    );
}

#[tokio::test]
async fn metadata_rule_matches_the_exact_value_and_nothing_else() {
    let framework = framework_with_policy(TtlPolicy::MetadataRule(vec![TtlRule {
        key: "tier".to_string(),
        value: json!("gold"),
        ttl_seconds: 7200,
    }]))
    .await;

    assert_eq!(
        framework
            .evaluate_ttl_policy("c", &HashMap::from([("tier".to_string(), json!("gold"))]))
            .await,
        Some(7200),
        "a metadata value equal to the rule value must select the rule"
    );
    assert_eq!(
        framework
            .evaluate_ttl_policy("c", &HashMap::from([("tier".to_string(), json!("silver"))]))
            .await,
        None,
        "a different value under the same key must not select the rule"
    );
    assert_eq!(
        framework
            .evaluate_ttl_policy("c", &HashMap::from([("plan".to_string(), json!("gold"))]))
            .await,
        None,
        "a matching value under a different key must not select the rule"
    );
}

#[tokio::test]
async fn inherit_policy_lends_the_remaining_life_of_an_expiring_target() {
    let framework = framework_with_policy(TtlPolicy::Inherit).await;
    framework
        .inject_concept_with_ttl("target", HVec10240::random(), 3600)
        .await
        .unwrap();
    framework
        .inject_concept("child", HVec10240::random())
        .await
        .unwrap();
    framework.associate("child", "target", 0.9).await.unwrap();

    let Some(remaining) = framework
        .evaluate_ttl_policy("child", &HashMap::new())
        .await
    else {
        panic!("a concept associated with an expiring target must inherit a ttl");
    };
    assert!(
        (3598..=3600).contains(&remaining),
        "inherited ttl must be the target's expiry minus now, got {remaining}"
    );
}

#[tokio::test]
async fn inherit_policy_has_nothing_to_lend_when_the_target_is_permanent_or_absent() {
    let framework = framework_with_policy(TtlPolicy::Inherit).await;
    framework
        .inject_concept("lonely", HVec10240::random())
        .await
        .unwrap();
    assert_eq!(
        framework
            .evaluate_ttl_policy("lonely", &HashMap::new())
            .await,
        None,
        "a concept with no associations inherits nothing"
    );

    framework
        .inject_concept("permanent", HVec10240::random())
        .await
        .unwrap();
    framework
        .associate("lonely", "permanent", 0.5)
        .await
        .unwrap();
    assert_eq!(
        framework
            .evaluate_ttl_policy("lonely", &HashMap::new())
            .await,
        None,
        "a permanent target has no remaining life to lend"
    );
}

#[tokio::test]
async fn inherit_policy_does_not_inherit_from_an_expired_target() {
    // ttl 0 stamps expires_at with the same second the evaluator reads back, so
    // `exp > now` is the whole decision: an off-by-one (`>=`) would hand out
    // Some(0) and an inverted comparison would underflow on `exp - now`.
    let framework = framework_with_policy(TtlPolicy::Inherit).await;
    framework
        .inject_concept_with_ttl("stale", HVec10240::random(), 0)
        .await
        .unwrap();
    framework
        .inject_concept("child", HVec10240::random())
        .await
        .unwrap();
    framework.associate("child", "stale", 0.9).await.unwrap();

    assert_eq!(
        framework
            .evaluate_ttl_policy("child", &HashMap::new())
            .await,
        None,
        "a target whose expiry has arrived must not pass on a ttl"
    );
}

#[tokio::test]
async fn inject_text_with_ttl_stores_the_text_and_the_expiry() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    framework
        .inject_text_with_ttl("doc", "semantic memory text", 3600)
        .await
        .unwrap();
    let concept = framework.get_concept("doc").await.unwrap().unwrap();
    assert_eq!(
        concept.metadata.get("searchable_text"),
        Some(&json!("semantic memory text")),
        "the source text must be stored for text search"
    );
    let expires_at = concept.expires_at.unwrap_or(0);
    let now = crate::singularity::unix_now_secs();
    assert!(
        (now + 3598..=now + 3600).contains(&expires_at),
        "expires_at must be now + ttl, got {expires_at} against now {now}"
    );

    assert!(
        framework
            .inject_text_with_ttl("", "rejected", 60)
            .await
            .is_err(),
        "an empty concept ID must not be accepted by the text+ttl path"
    );
}
