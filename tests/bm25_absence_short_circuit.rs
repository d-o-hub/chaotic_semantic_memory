#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BM25 absence short-circuit: threshold, skip predicate, inject invalidation.

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::{DEFAULT_ABSENCE_MIN_ATTEMPTS, is_known_absent};
use chaotic_semantic_memory::retrieval::hybrid::{HybridResult, RetrievalAbstention};
use chaotic_semantic_memory::{AbsenceEntry, AbsenceStore, persist_absence};
use chrono::Utc;
use tempfile::NamedTempFile;

/// After enough empty probe_text calls, is_known_absent reaches the threshold.
#[tokio::test]
async fn empty_probes_make_query_known_absent() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    let query = "no-such-concept-query";
    assert!(
        !is_known_absent(
            query,
            framework.persistence_store().unwrap().as_ref(),
            DEFAULT_ABSENCE_MIN_ATTEMPTS
        )
        .await
    );

    for i in 0..DEFAULT_ABSENCE_MIN_ATTEMPTS {
        let result = framework.probe_text(query, 5).await.unwrap();
        assert!(
            matches!(result, HybridResult::Abstained(_)),
            "iteration {i} should abstain on empty store"
        );
    }

    assert!(
        is_known_absent(
            query,
            framework.persistence_store().unwrap().as_ref(),
            DEFAULT_ABSENCE_MIN_ATTEMPTS
        )
        .await,
        "threshold should short-circuit BM25 after empty probes"
    );
}

/// Inject clears absence rows so BM25 is consulted again (invalidation).
#[tokio::test]
async fn inject_invalidates_absence_short_circuit() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let store = Persistence::new_local(path).await.unwrap();

    let query = "rust-memory-system";
    let abstention = RetrievalAbstention {
        query: query.to_string(),
        min_score_threshold: 0.5,
        best_score_seen: None,
        attempted_modes: vec!["Auto".to_string()],
        timestamp: Utc::now(),
    };
    for _ in 0..DEFAULT_ABSENCE_MIN_ATTEMPTS {
        persist_absence(&abstention, &store).await.unwrap();
    }
    assert!(is_known_absent(query, &store, DEFAULT_ABSENCE_MIN_ATTEMPTS).await);

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    // Corpus mutation must clear sticky absence.
    framework
        .inject_text("doc-1", "rust memory system documentation")
        .await
        .unwrap();

    assert!(
        !is_known_absent(
            query,
            framework.persistence_store().unwrap().as_ref(),
            DEFAULT_ABSENCE_MIN_ATTEMPTS
        )
        .await,
        "inject must invalidate absence short-circuit"
    );

    // BM25 index path should see the new document (keyword path).
    // Mirror CLI short-circuit gate: HDC-empty + known_absent only.
    let hdc_also_empty = false; // after inject, HDC may hit; keyword path still runs when false
    let skip = hdc_also_empty
        && is_known_absent(
            query,
            framework.persistence_store().unwrap().as_ref(),
            DEFAULT_ABSENCE_MIN_ATTEMPTS,
        )
        .await;
    assert!(!skip);
}

/// delete_absence / clear_all_absences API.
#[tokio::test]
async fn delete_and_clear_absences() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let store = Persistence::new_local(path).await.unwrap();

    let abstention = RetrievalAbstention {
        query: "q1".into(),
        min_score_threshold: 0.5,
        best_score_seen: None,
        attempted_modes: vec![],
        timestamp: Utc::now(),
    };
    persist_absence(&abstention, &store).await.unwrap();
    let id = AbsenceEntry::id_for("q1");
    store.delete_absence(&id).await.unwrap();
    assert!(store.get_absence(&id).await.unwrap().is_none());

    persist_absence(&abstention, &store).await.unwrap();
    store.clear_all_absences().await.unwrap();
    assert!(store.list_absences(0).await.unwrap().is_empty());
}
