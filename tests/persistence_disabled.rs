//! ADR-0094: persistence-disabled configuration cannot return false success.
//!
//! These tests are compiled and run only when the `persistence` feature is
//! disabled (`--no-default-features`), where `with_local_db`/`with_turso`
//! record their configuration and `FrameworkBuilder::build()` must reject it
//! with `UnsupportedOperation` instead of silently building an in-memory
//! framework.

#![cfg(not(feature = "persistence"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::{FrameworkBuilder, MemoryError};

#[tokio::test]
async fn build_with_configured_db_fails_when_persistence_disabled() {
    let result = FrameworkBuilder::new()
        .with_local_db("/tmp/should-not-open.db")
        .build()
        .await;
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("configured DB must be rejected when persistence is disabled"),
    };
    assert!(
        matches!(
            err,
            MemoryError::UnsupportedOperation(ref msg) if msg.contains("persistence is disabled")
        ),
        "expected UnsupportedOperation mentioning disabled persistence, got: {err}"
    );
}

#[tokio::test]
async fn build_with_configured_turso_fails_when_persistence_disabled() {
    let result = FrameworkBuilder::new()
        .with_turso("libsql://example.turso.io", "secret")
        .build()
        .await;
    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("configured Turso must be rejected when persistence is disabled"),
    };
    assert!(
        matches!(
            err,
            MemoryError::UnsupportedOperation(ref msg) if msg.contains("persistence is disabled")
        ),
        "expected UnsupportedOperation mentioning disabled persistence, got: {err}"
    );
}

#[tokio::test]
async fn build_without_db_succeeds_when_persistence_disabled() {
    // The common memory-only path (no DB configured) must still build.
    let fw = FrameworkBuilder::new()
        .without_persistence()
        .build()
        .await
        .expect("memory-only build must succeed when persistence is disabled");
    fw.inject_concept("c1", chaotic_semantic_memory::HVec10240::random())
        .await
        .expect("inject on memory-only framework");
}
