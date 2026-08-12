//! Unit coverage for `is_known_absent` boundary semantics (M1).
//!
//! Lives in a separate `#[cfg(test)]` module so `src/retrieval/bm25/tests.rs`
//! stays under the 500-LOC gate. `AbsenceStore`/`AbsenceEntry` are private to
//! the crate (`mod bridge_persistence`), so these cannot live in `tests/`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::is_known_absent;
use crate::bridge_persistence::{AbsenceEntry, AbsenceStore};
use std::collections::HashMap;

struct StubStore {
    entries: HashMap<String, AbsenceEntry>,
}

#[async_trait::async_trait]
impl AbsenceStore for StubStore {
    async fn get_absence(&self, id: &str) -> csm_core_lib::error::Result<Option<AbsenceEntry>> {
        Ok(self.entries.get(id).cloned())
    }

    async fn upsert_absence(&self, _entry: &AbsenceEntry) -> csm_core_lib::error::Result<()> {
        Ok(())
    }

    async fn list_absences(
        &self,
        _min_attempts: u32,
    ) -> csm_core_lib::error::Result<Vec<AbsenceEntry>> {
        Ok(self.entries.values().cloned().collect())
    }
}

fn entry(query: &str, attempt_count: u32) -> AbsenceEntry {
    AbsenceEntry {
        id: AbsenceEntry::id_for(query),
        query: query.to_string(),
        normalized_query: AbsenceEntry::normalize(query),
        attempt_count,
        last_threshold: 0.0,
        best_score_ever: None,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn is_known_absent_respects_min_attempts() {
    let query = "some query";
    let store = StubStore {
        entries: [(AbsenceEntry::id_for(query), entry(query, 2))]
            .into_iter()
            .collect(),
    };

    // Unknown query id → false.
    assert!(!is_known_absent("unknown query", &store, 1).await);
    // attempt_count == min_attempts → true.
    assert!(is_known_absent(query, &store, 2).await);
    // attempt_count == min_attempts - 1 → false.
    assert!(!is_known_absent(query, &store, 3).await);
}
