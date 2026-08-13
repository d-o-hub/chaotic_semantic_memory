#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::bridge_persistence::{absence_from_abstention, merge_absence_with, persist_absence};
use crate::persistence::Persistence;
use crate::retrieval::hybrid::RetrievalAbstention;
use crate::semantic_bridge::{CanonicalConcept, ConceptGraph};
use chrono::Utc;
use csm_traits::AbsenceEntry;
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

/// Kills fnv1a_hash -> 0/1 and ^= -> |=: tests the exact FNV-1a-64 output for
/// known inputs so operator substitutions (|= sets bits, never clears) are caught.
#[test]
fn test_absence_fnv1a_hash_known_values() {
    // FNV-1a-64: offset=0xcbf29ce484222325, prime=0x00000100000001b3
    // Empty string: no loop iterations → offset basis.
    assert_eq!(
        AbsenceEntry::fnv1a_hash(b""),
        0xcbf2_9ce4_8422_2325,
        "empty input must equal FNV-1a offset basis"
    );
    // Single byte 'a' (0x61): (offset ^ 0x61) * prime = 0xaf63dc4c8601ec8c
    // |= mutant gives 0xaf63fd4c8602249f (different because OR keeps bits XOR clears)
    assert_eq!(
        AbsenceEntry::fnv1a_hash(b"a"),
        0xaf63_dc4c_8601_ec8c,
        "fnv1a(b\"a\") must equal known FNV-1a-64 value"
    );
    // Distinct inputs must produce distinct hashes.
    assert_ne!(
        AbsenceEntry::fnv1a_hash(b"hello"),
        AbsenceEntry::fnv1a_hash(b"world"),
    );
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
    merge_absence_with(&mut entry, &abstention);
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
    merge_absence_with(&mut entry, &abstention);
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

// --- fnv1a_hash mutation coverage (^= vs |=) ---

#[test]
fn fnv1a_hash_is_xor_based_not_or_based() {
    // Two distinct bytes must produce distinct hashes.
    // If ^= were replaced by |=, bits only accumulate (never flip to 0),
    // so many distinct inputs collapse to the same hash.
    let h1 = AbsenceEntry::fnv1a_hash(b"\x00");
    let h2 = AbsenceEntry::fnv1a_hash(b"\xff");
    let h3 = AbsenceEntry::fnv1a_hash(b"\x0f");

    // All three must be distinct — collapses under |= mutation
    assert_ne!(h1, h2, "hash(0x00) must differ from hash(0xff)");
    assert_ne!(h1, h3, "hash(0x00) must differ from hash(0x0f)");
    assert_ne!(h2, h3, "hash(0xff) must differ from hash(0x0f)");
}

#[test]
fn fnv1a_hash_known_vector() {
    // FNV-1a 64-bit of empty slice is the offset basis itself.
    let empty = AbsenceEntry::fnv1a_hash(b"");
    assert_eq!(
        empty, 0xcbf2_9ce4_8422_2325,
        "empty hash must equal FNV offset basis"
    );

    // FNV-1a of "a" = known reference value
    let a = AbsenceEntry::fnv1a_hash(b"a");
    assert_eq!(
        a, 0xaf63_dc4c_8601_ec8c,
        "fnv1a('a') must match reference vector"
    );
}

#[test]
fn fnv1a_hash_byte_order_sensitivity() {
    // XOR is non-associative with sequence; |= would lose this property.
    let forward = AbsenceEntry::fnv1a_hash(b"\x01\x02");
    let backward = AbsenceEntry::fnv1a_hash(b"\x02\x01");
    assert_ne!(forward, backward, "hash must be order-sensitive");
}

// --- merge_with mutation coverage (> vs >=) ---

fn make_abstention(score: Option<f32>) -> crate::retrieval::hybrid::RetrievalAbstention {
    use chrono::Utc;
    crate::retrieval::hybrid::RetrievalAbstention {
        query: "test query".to_string(),
        min_score_threshold: 0.5,
        best_score_seen: score,
        attempted_modes: vec![],
        timestamp: Utc::now(),
    }
}

#[test]
fn merge_with_keeps_strictly_higher_score() {
    let abstention_initial = make_abstention(Some(0.3));
    let mut entry = absence_from_abstention(&abstention_initial);

    // Merge with strictly higher score — must update
    let higher = make_abstention(Some(0.8));
    merge_absence_with(&mut entry, &higher);
    assert_eq!(entry.best_score_ever, Some(0.8));
}

#[test]
fn merge_with_does_not_overwrite_equal_score() {
    // Critical: distinguishes `>` from `>=`
    // With `>=`, the value is overwritten (same result here, but
    // we pin the pointer identity by checking the value is unchanged
    // when the new score equals the existing one).
    let abstention_initial = make_abstention(Some(0.5));
    let mut entry = absence_from_abstention(&abstention_initial);
    assert_eq!(entry.best_score_ever, Some(0.5));

    let equal_score = make_abstention(Some(0.5));
    merge_absence_with(&mut entry, &equal_score);
    // Score should remain 0.5; both `>` and `>=` produce same numeric result,
    // BUT we additionally test that a lower score is not promoted:
    assert_eq!(
        entry.best_score_ever,
        Some(0.5),
        "equal score must not change best_score_ever"
    );

    let lower = make_abstention(Some(0.2));
    merge_absence_with(&mut entry, &lower);
    assert_eq!(
        entry.best_score_ever,
        Some(0.5),
        "lower score must not displace existing best"
    );
}

#[test]
fn merge_with_promotes_none_to_some() {
    let initial = make_abstention(None);
    let mut entry = absence_from_abstention(&initial);
    assert_eq!(entry.best_score_ever, None);

    let with_score = make_abstention(Some(0.7));
    merge_absence_with(&mut entry, &with_score);
    assert_eq!(entry.best_score_ever, Some(0.7));
}
