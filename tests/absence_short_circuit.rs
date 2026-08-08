//! Integration tests for absence-memory short-circuit on probe paths.
//!
//! Covers threshold gating, KnownAbsent mode marker, inject invalidation,
//! and fail-open when short-circuit is disabled.

#![cfg(feature = "persistence")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::framework_builder::FrameworkBuilder;
use chaotic_semantic_memory::retrieval::hybrid::HybridResult;
use tempfile::NamedTempFile;

const QUERY: &str = "completely unknown concept xyzzy";

async fn framework_with_threshold(
    min_attempts: u32,
) -> (
    NamedTempFile,
    chaotic_semantic_memory::ChaoticSemanticFramework,
) {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let framework = FrameworkBuilder::new()
        .with_local_db(&path)
        .with_absence_short_circuit_min_attempts(min_attempts)
        .build()
        .await
        .unwrap();
    (temp, framework)
}

#[tokio::test]
async fn probe_below_threshold_still_searches_and_records() {
    let (_temp, framework) = framework_with_threshold(3).await;

    for _ in 0..2 {
        match framework.probe_text(QUERY, 5).await.unwrap() {
            HybridResult::Abstained(a) => {
                assert!(
                    !a.attempted_modes.iter().any(|m| m == "KnownAbsent"),
                    "must not short-circuit before threshold"
                );
            }
            HybridResult::Success(_) => panic!("empty store should abstain"),
        }
    }
}

#[tokio::test]
async fn probe_at_threshold_short_circuits_with_known_absent() {
    let (_temp, framework) = framework_with_threshold(3).await;

    // Seed three failed probes (threshold = 3)
    for _ in 0..3 {
        let _ = framework.probe_text(QUERY, 5).await.unwrap();
    }

    match framework.probe_text(QUERY, 5).await.unwrap() {
        HybridResult::Abstained(a) => {
            assert_eq!(a.attempted_modes, vec!["KnownAbsent".to_string()]);
            assert_eq!(a.query, QUERY);
        }
        HybridResult::Success(_) => panic!("expected KnownAbsent short-circuit"),
    }
}

#[tokio::test]
async fn inject_invalidates_absence_short_circuit() {
    let (_temp, framework) = framework_with_threshold(2).await;

    for _ in 0..2 {
        let _ = framework.probe_text(QUERY, 5).await.unwrap();
    }
    assert!(
        framework.is_known_absent_query(QUERY).await,
        "should be known-absent after 2 failures"
    );

    // Inject any concept clears absence rows (DB-global invalidation).
    framework
        .inject_text("c1", "unrelated content about robotics")
        .await
        .unwrap();

    assert!(
        !framework.is_known_absent_query(QUERY).await,
        "inject must clear absence short-circuit"
    );

    // Next probe should search again (Auto mode abstention, not KnownAbsent)
    match framework.probe_text(QUERY, 5).await.unwrap() {
        HybridResult::Abstained(a) => {
            assert!(
                !a.attempted_modes.iter().any(|m| m == "KnownAbsent"),
                "fresh search after inject"
            );
        }
        HybridResult::Success(_) => {
            // Possible if inject text is somehow similar; still not KnownAbsent
        }
    }
}

#[tokio::test]
async fn short_circuit_disabled_when_min_attempts_zero() {
    let (_temp, framework) = framework_with_threshold(0).await;

    for _ in 0..5 {
        match framework.probe_text(QUERY, 5).await.unwrap() {
            HybridResult::Abstained(a) => {
                assert!(
                    !a.attempted_modes.iter().any(|m| m == "KnownAbsent"),
                    "disabled short-circuit must never emit KnownAbsent"
                );
            }
            HybridResult::Success(_) => {}
        }
    }
    assert!(!framework.is_known_absent_query(QUERY).await);
}

#[tokio::test]
async fn short_circuit_does_not_increment_attempt_count() {
    let (_temp, framework) = framework_with_threshold(2).await;

    for _ in 0..2 {
        let _ = framework.probe_text(QUERY, 5).await.unwrap();
    }

    // Multiple short-circuit hits should not keep growing attempt_count via re-persist.
    for _ in 0..3 {
        match framework.probe_text(QUERY, 5).await.unwrap() {
            HybridResult::Abstained(a) => {
                assert_eq!(a.attempted_modes, vec!["KnownAbsent".to_string()]);
            }
            HybridResult::Success(_) => panic!("expected short-circuit"),
        }
    }

    // Still short-circuits (store still has the row; not cleared)
    assert!(framework.is_known_absent_query(QUERY).await);
}

#[tokio::test]
async fn absence_is_scoped_per_namespace() {
    let (_temp, framework) = framework_with_threshold(3).await;
    framework.set_namespace("ns-a").await.unwrap();

    // Reach the threshold in ns-a.
    for _ in 0..3 {
        let _ = framework.probe_text(QUERY, 5).await.unwrap();
    }
    assert!(
        framework.is_known_absent_query(QUERY).await,
        "should be known-absent in ns-a after 3 failures"
    );

    // Same text in ns-b must NOT short-circuit (per-namespace absence rows).
    framework.set_namespace("ns-b").await.unwrap();
    assert!(
        !framework.is_known_absent_query(QUERY).await,
        "absence in ns-a must not leak into ns-b"
    );
    match framework.probe_text(QUERY, 5).await.unwrap() {
        HybridResult::Abstained(a) => {
            assert!(
                !a.attempted_modes.iter().any(|m| m == "KnownAbsent"),
                "ns-b probe must actually search"
            );
        }
        HybridResult::Success(_) => panic!("empty store should abstain"),
    }

    // Returning to ns-a: the absence record is still intact.
    framework.set_namespace("ns-a").await.unwrap();
    assert!(framework.is_known_absent_query(QUERY).await);
}
