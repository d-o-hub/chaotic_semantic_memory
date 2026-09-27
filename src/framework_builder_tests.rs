//! Unit tests for the `FrameworkBuilder` disabled-persistence contracts and parameter clamping.
//!
//! Kept in a separate `#[cfg(test)]` module so `src/framework_builder.rs`
//! stays under the 500-LOC gate while the `--lib` mutation profile still
//! compiles and runs these tests (integration tests under `tests/` do not
//! run under `--lib`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[cfg(not(feature = "persistence"))]
#[tokio::test]
async fn builder_rejects_configured_db_when_persistence_disabled() {
    // ADR-0094: recorded DB config must fail build(), killing
    // with_local_db -> Default::default() and || -> && mutants.
    let err = crate::FrameworkBuilder::new()
        .with_local_db("/tmp/x.db")
        .build()
        .await
        .err()
        .expect("configured DB must be rejected");
    assert!(format!("{err}").contains("persistence is disabled"));
}

#[cfg(not(feature = "persistence"))]
#[tokio::test]
async fn builder_rejects_configured_turso_when_persistence_disabled() {
    // Kills with_turso -> Default::default(): db_token alone must fail.
    let err = crate::FrameworkBuilder::new()
        .with_turso("libsql://x", "tok")
        .build()
        .await
        .err()
        .expect("configured Turso must be rejected");
    assert!(format!("{err}").contains("persistence is disabled"));
}

#[test]
fn test_max_associations_per_concept_clamping() {
    let limit = crate::framework_validation::MAX_ASSOCIATIONS_PER_CONCEPT_LIMIT;

    let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit + 1);
    assert_eq!(builder.config.max_associations_per_concept, Some(limit));

    let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit);
    assert_eq!(builder.config.max_associations_per_concept, Some(limit));

    let builder = FrameworkBuilder::new().with_max_associations_per_concept(limit - 1);
    assert_eq!(builder.config.max_associations_per_concept, Some(limit - 1));
}

#[test]
fn test_with_chaos_strength_clamping() {
    let b = FrameworkBuilder::new().with_chaos_strength(1.5);
    assert!((b.config.chaos_strength - 1.0).abs() < f32::EPSILON);

    let b = FrameworkBuilder::new().with_chaos_strength(-0.5);
    assert!((b.config.chaos_strength - 0.0).abs() < f32::EPSILON);

    let b = FrameworkBuilder::new().with_chaos_strength(f32::NAN);
    assert!((b.config.chaos_strength - 0.0).abs() < f32::EPSILON);

    let b = FrameworkBuilder::new().with_chaos_strength(f32::INFINITY);
    assert!((b.config.chaos_strength - 0.0).abs() < f32::EPSILON);

    let b = FrameworkBuilder::new().with_chaos_strength(0.5);
    assert!((b.config.chaos_strength - 0.5).abs() < f32::EPSILON);
}

#[tokio::test]
async fn build_default_bruteforce_ok() {
    let fw = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await
        .expect("default BruteForce backend must build");
    fw.inject_concept("c1", csm_core_lib::HVec10240::random())
        .await
        .expect("inject on default backend");
}
