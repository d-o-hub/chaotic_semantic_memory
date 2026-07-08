//! Integration tests for AbsenceEntry, AbsenceStore, and persist_absence.
//! Extracted from src/bridge_persistence.rs inline test module.

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::retrieval::hybrid::RetrievalAbstention;
use chaotic_semantic_memory::{AbsenceEntry, persist_absence};
use chrono::Utc;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_absence_id_generation() {
    let q1 = "  Test Query  ";
    let q2 = "test query";
    let id1 = AbsenceEntry::id_for(q1);
    let id2 = AbsenceEntry::id_for(q2);
    assert_eq!(id1, id2);
    assert!(id1.starts_with("absence:"));
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

    // Create
    let mut abstention = abstention;
    abstention.best_score_seen = Some(0.1);
    let entry = persist_absence(&abstention, &persistence).await.unwrap();
    assert_eq!(entry.attempt_count, 1);
    assert!((entry.best_score_ever.unwrap() - 0.1).abs() < f32::EPSILON);

    // Update with higher score
    let mut abstention2 = abstention.clone();
    abstention2.best_score_seen = Some(0.4);
    let entry2 = persist_absence(&abstention2, &persistence).await.unwrap();
    assert_eq!(entry2.attempt_count, 2);
    assert!((entry2.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON);

    // Update with lower score — best_score_ever stays at 0.4
    let mut abstention3 = abstention.clone();
    abstention3.best_score_seen = Some(0.2);
    let entry3 = persist_absence(&abstention3, &persistence).await.unwrap();
    assert_eq!(entry3.attempt_count, 3);
    assert!((entry3.best_score_ever.unwrap() - 0.4).abs() < f32::EPSILON);
}

#[test]
fn test_normalize_trims_and_lowercases() {
    assert_eq!(AbsenceEntry::normalize("  Hello World  "), "hello world");
    assert_eq!(AbsenceEntry::normalize("UPPER"), "upper");
    assert_eq!(AbsenceEntry::normalize("  "), "");
}

fn make_entry(best: Option<f32>) -> AbsenceEntry {
    AbsenceEntry {
        id: "absence:abc".into(),
        query: "test".into(),
        normalized_query: "test".into(),
        attempt_count: 1,
        last_threshold: 0.5,
        best_score_ever: best,
        first_seen: Utc::now(),
        last_seen: Utc::now(),
    }
}

fn make_abstention(score: Option<f32>, threshold: f32) -> RetrievalAbstention {
    RetrievalAbstention {
        query: "test".into(),
        min_score_threshold: threshold,
        best_score_seen: score,
        attempted_modes: vec![],
        timestamp: Utc::now(),
    }
}

#[test]
fn test_merge_score_scenarios() {
    let mut e = make_entry(None);
    e.merge_with(&make_abstention(Some(0.3), 0.6));
    assert_eq!(e.attempt_count, 2);
    assert!((e.best_score_ever.unwrap() - 0.3).abs() < f32::EPSILON);

    let mut e = make_entry(Some(0.2));
    e.merge_with(&make_abstention(Some(0.5), 0.5));
    assert!((e.best_score_ever.unwrap() - 0.5).abs() < f32::EPSILON);

    let mut e = make_entry(Some(0.3));
    e.merge_with(&make_abstention(Some(0.3), 0.5));
    assert!((e.best_score_ever.unwrap() - 0.3).abs() < f32::EPSILON);

    let mut e = make_entry(Some(0.5));
    e.merge_with(&make_abstention(Some(0.1), 0.5));
    assert!((e.best_score_ever.unwrap() - 0.5).abs() < f32::EPSILON);
}
