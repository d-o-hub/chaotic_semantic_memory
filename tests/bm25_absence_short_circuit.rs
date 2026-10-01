#![cfg(feature = "persistence")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chaotic_semantic_memory::prelude::FrameworkBuilder;
use chaotic_semantic_memory::retrieval::hybrid::HybridResult;
use tempfile::tempdir;

#[tokio::test]
async fn known_absent_query_short_circuits_after_three_abstentions() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("absence.db");
    let db_path = db_path.to_str().unwrap();

    let framework = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    let query = "a-query-that-abstains-repeatedly";

    // First three probes abstain (no concepts exist) and persist absence attempts.
    for _ in 0..3 {
        match framework.probe_text(query, 5).await.unwrap() {
            HybridResult::Abstained(_) => {}
            HybridResult::Success(_) => panic!("expected abstention before any concepts exist"),
        }
    }

    // Fourth probe must short-circuit without re-running retrieval.
    match framework.probe_text(query, 5).await.unwrap() {
        HybridResult::Abstained(abstention) => {
            assert_eq!(
                abstention.attempted_modes,
                vec!["AbsenceShortCircuit".to_string()],
                "fourth probe of a known-absent query must short-circuit"
            );
        }
        HybridResult::Success(_) => panic!("known-absent query must abstain"),
    }
}

#[tokio::test]
async fn short_circuit_is_per_query_not_global() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("absence-per-query.db");
    let db_path = db_path.to_str().unwrap();

    let framework = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    // Exhaust absence attempts for one query.
    for _ in 0..3 {
        let _ = framework.probe_text("absent-query", 5).await.unwrap();
    }

    // Inject a concept reachable by text and probe a *different* query — must
    // succeed, proving the short-circuit is per-query rather than global.
    framework
        .inject_text("positive-control", "positive control phrase")
        .await
        .unwrap();

    match framework
        .probe_text("positive control phrase", 5)
        .await
        .unwrap()
    {
        HybridResult::Success(results) => assert!(!results.is_empty()),
        HybridResult::Abstained(_) => panic!("different query must not short-circuit"),
    }
}

#[tokio::test]
async fn inserting_matching_content_restores_retrieval() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("absence-invalidation.db");
    let db_path = db_path.to_str().unwrap();

    let framework = FrameworkBuilder::new()
        .with_local_db(db_path)
        .build()
        .await
        .unwrap();

    let query = "a-query-that-becomes-answerable";

    // Three abstentions make the query known-absent; the fourth probe
    // short-circuits without running retrieval.
    for _ in 0..3 {
        match framework.probe_text(query, 5).await.unwrap() {
            HybridResult::Abstained(_) => {}
            HybridResult::Success(_) => panic!("expected abstention before any concepts exist"),
        }
    }
    match framework.probe_text(query, 5).await.unwrap() {
        HybridResult::Abstained(abstention) => assert_eq!(
            abstention.attempted_modes,
            vec!["AbsenceShortCircuit".to_string()],
            "the query must be known-absent at this point"
        ),
        HybridResult::Success(_) => panic!("known-absent query must short-circuit"),
    }

    // Content the query matches makes the persisted record stale: the insert
    // advances the namespace revision, and a record is only authoritative for
    // the content it was observed against, so retrieval must resume.
    framework.inject_text("answer", query).await.unwrap();

    match framework.probe_text(query, 5).await.unwrap() {
        HybridResult::Success(results) => assert!(
            !results.is_empty(),
            "the injected concept must be retrieved, not suppressed by a stale absence record"
        ),
        HybridResult::Abstained(abstention) => panic!(
            "retrieval must resume after matching content exists, got {:?}",
            abstention.attempted_modes
        ),
    }

    // The successful probe also dropped the record, so the next probe cannot
    // fall back into the short-circuit.
    match framework.probe_text(query, 5).await.unwrap() {
        HybridResult::Success(results) => assert!(!results.is_empty()),
        HybridResult::Abstained(abstention) => panic!(
            "the absence record must have been invalidated, got {:?}",
            abstention.attempted_modes
        ),
    }
}
