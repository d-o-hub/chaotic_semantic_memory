//! Unit tests for the `FrameworkBuilder` disabled-persistence contracts.
//!
//! Kept in a separate `#[cfg(test)]` module so `src/framework_builder.rs`
//! stays under the 500-LOC gate while the `--lib` mutation profile still
//! compiles and runs these tests (integration tests under `tests/` do not
//! run under `--lib`). These kill the `with_local_db`/`with_turso`
//! `Default::default()` and the `|| -> &&` mutants under `--no-default-features`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
