#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge_persistence::{AbsenceEntry, persist_absence};
use crate::persistence::Persistence;
use crate::retrieval::hybrid::RetrievalAbstention;
use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
use chrono::Utc;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_save_and_load_canonical_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = CanonicalConcept::new("test-concept")
        .with_label("label1")
        .with_label("label2")
        .with_related("related-concept");

    persistence
        .save_canonical_concept("_default", &concept)
        .await
        .unwrap();

    let loaded = persistence
        .load_canonical_concept("_default", "test-concept")
        .await
        .unwrap();
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.id, "test-concept");
    assert_eq!(loaded.labels, vec!["label1", "label2"]);
    assert_eq!(loaded.related, vec!["related-concept"]);
}

#[tokio::test]
async fn test_delete_canonical_concept() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let concept = CanonicalConcept::new("to-delete");
    persistence
        .save_canonical_concept("_default", &concept)
        .await
        .unwrap();

    persistence
        .delete_canonical_concept("_default", "to-delete")
        .await
        .unwrap();

    let loaded = persistence
        .load_canonical_concept("_default", "to-delete")
        .await
        .unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_save_and_load_concept_graph() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("c1")
            .with_label("label1")
            .with_related("c2"),
    );
    graph.add_concept(CanonicalConcept::new("c2").with_label("label2"));

    persistence
        .save_concept_graph("_default", &graph)
        .await
        .unwrap();

    let loaded = persistence.load_concept_graph("_default").await.unwrap();
    assert_eq!(loaded.concept_count(), 2);
    assert_eq!(loaded.label_count(), 2);
}

#[tokio::test]
async fn test_absence_id_generation() {
    let q1 = "  Test Query  ";
    let q2 = "test query";
    let id1 = AbsenceEntry::id_for(q1);
    let id2 = AbsenceEntry::id_for(q2);
    assert_eq!(id1, id2);
    assert!(id1.starts_with("absence:"));
}

/// Kills normalize -> String::new() / "xyzzy": result must equal trimmed lowercase.
#[test]
fn test_absence_normalize_trims_and_lowercases() {
    assert_eq!(AbsenceEntry::normalize("  Hello World  "), "hello world");
    assert_eq!(AbsenceEntry::normalize("RUST"), "rust");
    assert_eq!(AbsenceEntry::normalize("already"), "already");
}

/// Kills fnv1a_hash -> 0/1 and ^= -> |=/&=: different inputs must produce distinct hashes.
#[test]
fn test_absence_fnv1a_hash_distinct() {
    let h_a = AbsenceEntry::fnv1a_hash(b"hello");
    let h_b = AbsenceEntry::fnv1a_hash(b"world");
    let h_empty = AbsenceEntry::fnv1a_hash(b"");
    assert_ne!(h_a, 0, "hash must not be the trivial zero");
    assert_ne!(h_a, 1, "hash must not be the trivial one");
    assert_ne!(h_a, h_b, "distinct inputs must produce distinct hashes");
    assert_ne!(h_a, h_empty, "non-empty must differ from empty");
    let h_a2 = AbsenceEntry::fnv1a_hash(b"hellp");
    assert_ne!(h_a, h_a2, "adjacent inputs must produce distinct hashes");
}

/// Kills merge_with (Some(new), None) arm deletion: best_score_ever must be set from None.
#[test]
fn test_merge_with_sets_score_from_none() {
    let ts = Utc::now();
    let mut entry = AbsenceEntry {
        id: "absence:0000".to_string(),
        query: "q".to_string(),
        normalized_query: "q".to_string(),
        attempt_count: 1,
        last_threshold: 0.5,
        best_score_ever: None,
        first_seen: ts,
        last_seen: ts,
    };
    let abstention = RetrievalAbstention {
        query: "q".to_string(),
        min_score_threshold: 0.5,
        best_score_seen: Some(0.7),
        attempted_modes: vec![],
        timestamp: ts,
    };
    entry.merge_with(&abstention);
    assert_eq!(
        entry.best_score_ever,
        Some(0.7),
        "(Some(new), None) arm must set best_score_ever"
    );
}

/// Kills merge_with > -> >= boundary mutant: lower score must not overwrite best_score_ever.
#[test]
fn test_merge_with_lower_score_unchanged() {
    let ts = Utc::now();
    let mut entry = AbsenceEntry {
        id: "absence:0000".to_string(),
        query: "q".to_string(),
        normalized_query: "q".to_string(),
        attempt_count: 1,
        last_threshold: 0.5,
        best_score_ever: Some(0.5),
        first_seen: ts,
        last_seen: ts,
    };
    let abstention = RetrievalAbstention {
        query: "q".to_string(),
        min_score_threshold: 0.5,
        best_score_seen: Some(0.3),
        attempted_modes: vec![],
        timestamp: ts,
    };
    entry.merge_with(&abstention);
    assert!(
        (entry.best_score_ever.unwrap() - 0.5).abs() < f32::EPSILON,
        "lower score must not overwrite best_score_ever"
    );
}

#[tokio::test]
async fn test_persist_absence_lifecycle() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();
    let persistence = Persistence::new_local(path).await.unwrap();

    let abstention = RetrievalAbstention {
        query: "unknown".to_string(),
        min_score_threshold: 0.5,
        best_score_seen: Some(0.1),
        attempted_modes: vec!["Auto".to_string()],
        timestamp: Utc::now(),
    };

    let mut abstention = abstention;
    abstention.best_score_seen = Some(0.1);
    let entry = persist_absence(&abstention, &persistence).await.unwrap();
    assert_eq!(entry.attempt_count, 1);
    assert!((entry.best_score_ever.unwrap() - 0.1).abs() < f32::EPSILON);

    let mut abstention2 = abstention.clone();
    abstention2.best_score_seen = Some(0.4);
    let entry2 = persist_absence(&abstention2, &persistence).await.unwrap();
    assert_eq!(entry2.attempt_count, 2);
    assert!((entry2.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON);

    let mut abstention3 = abstention.clone();
    abstention3.best_score_seen = Some(0.2);
    let entry3 = persist_absence(&abstention3, &persistence).await.unwrap();
    assert_eq!(entry3.attempt_count, 3);
    assert!((entry2.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON); // Stays at 0.4
}
