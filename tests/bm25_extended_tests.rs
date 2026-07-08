// Integration tests for BM25 is_known_absent functionality.
// Extracted from src/retrieval/bm25/tests.rs to keep unit test file under 500 LOC.

#![cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]

use chaotic_semantic_memory::{AbsenceEntry, AbsenceStore, MemoryError, Result};
use std::sync::Mutex;

/// A minimal mock that returns a pre-configured response for `get_absence`.
struct MockAbsenceStore {
    response: Mutex<Option<Result<Option<AbsenceEntry>>>>,
}

impl MockAbsenceStore {
    const fn new(response: Result<Option<AbsenceEntry>>) -> Self {
        Self {
            response: Mutex::new(Some(response)),
        }
    }
}

#[async_trait::async_trait]
impl AbsenceStore for MockAbsenceStore {
    async fn get_absence(&self, _id: &str) -> Result<Option<AbsenceEntry>> {
        self.response.lock().unwrap().take().unwrap_or(Ok(None))
    }

    async fn upsert_absence(&self, _entry: &AbsenceEntry) -> Result<()> {
        Ok(())
    }

    async fn list_absences(&self, _min_attempts: u32) -> Result<Vec<AbsenceEntry>> {
        Ok(Vec::new())
    }
}

fn make_entry(attempt_count: u32) -> AbsenceEntry {
    AbsenceEntry {
        id: AbsenceEntry::id_for("test"),
        query: "test".into(),
        normalized_query: "test".into(),
        attempt_count,
        last_threshold: 0.5,
        best_score_ever: None,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
    }
}

// Kill mutant: replace function body with `true`
// Store returns Ok(None) → must return false.
#[tokio::test]
async fn absent_returns_false_when_entry_not_found() {
    let store = MockAbsenceStore::new(Ok(None));
    assert!(
        !chaotic_semantic_memory::retrieval::bm25::is_known_absent("test", &store, 1).await,
        "should be false when store has no entry for the query"
    );
}

// Kill mutant: replace function body with `true`
// Store returns Err(...) → must return false (the `_ => false` arm).
#[tokio::test]
async fn absent_returns_false_on_store_error() {
    let store = MockAbsenceStore::new(Err(MemoryError::database("boom")));
    assert!(
        !chaotic_semantic_memory::retrieval::bm25::is_known_absent("test", &store, 1).await,
        "should be false when the store returns an error"
    );
}

// Kill mutant: replace function body with `false`, or delete Ok(Some) arm
// Entry exists with attempt_count >= min_attempts → must return true.
#[tokio::test]
async fn absent_returns_true_when_attempts_meet_threshold() {
    let store = MockAbsenceStore::new(Ok(Some(make_entry(5))));
    assert!(
        chaotic_semantic_memory::retrieval::bm25::is_known_absent("test", &store, 3).await,
        "should be true when attempt_count (5) >= min_attempts (3)"
    );
}

// Kill mutant: replace `>=` with `<`
// Entry exists but attempt_count < min_attempts → must return false.
#[tokio::test]
async fn absent_returns_false_when_attempts_below_threshold() {
    let store = MockAbsenceStore::new(Ok(Some(make_entry(2))));
    assert!(
        !chaotic_semantic_memory::retrieval::bm25::is_known_absent("test", &store, 5).await,
        "should be false when attempt_count (2) < min_attempts (5)"
    );
}

// Boundary: attempt_count == min_attempts exactly → must return true (>=, not >).
#[tokio::test]
async fn absent_returns_true_when_attempts_equal_threshold() {
    let store = MockAbsenceStore::new(Ok(Some(make_entry(3))));
    assert!(
        chaotic_semantic_memory::retrieval::bm25::is_known_absent("test", &store, 3).await,
        "should be true when attempt_count (3) == min_attempts (3)"
    );
}
