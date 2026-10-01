//! Unit coverage for `lookup_absence` boundary semantics (M1).
//!
//! Lives in a separate `#[cfg(test)]` module so the shim `src/retrieval/bm25.rs`
//! stays under the 500-LOC gate. `AbsenceStore`/`AbsenceEntry` are private to
//! the crate (`mod bridge_persistence`), so these cannot live in `tests/`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::lookup_absence;
use csm_traits::{AbsenceEntry, AbsenceStore};
use std::collections::HashMap;

const NS: &str = "_default";
const REVISION: u64 = 7;

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

    async fn delete_absence(&self, _id: &str) -> csm_core_lib::error::Result<()> {
        Ok(())
    }
}

fn entry(
    query: &str,
    attempt_count: u32,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceEntry {
    AbsenceEntry {
        id: AbsenceEntry::id_for(query),
        query: query.to_string(),
        normalized_query: AbsenceEntry::normalize(query),
        attempt_count,
        last_threshold: 0.0,
        best_score_ever: None,
        first_seen: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        namespace: namespace.to_string(),
        namespace_revision,
    }
}

fn store_with(entry: AbsenceEntry) -> StubStore {
    StubStore {
        entries: [(entry.id.clone(), entry)].into_iter().collect(),
    }
}

#[tokio::test]
async fn lookup_absence_respects_min_attempts() {
    let query = "some query";
    let store = store_with(entry(query, 2, NS, REVISION));

    // Unknown query id → nothing to skip and nothing to invalidate.
    let unknown = lookup_absence("unknown query", &store, 1, NS, REVISION).await;
    assert!(!unknown.short_circuit);
    assert!(unknown.entry_id.is_none());

    // attempt_count == min_attempts → short-circuit.
    let at_threshold = lookup_absence(query, &store, 2, NS, REVISION).await;
    assert!(at_threshold.short_circuit);
    assert_eq!(
        at_threshold.entry_id.as_deref(),
        Some(query_id(query).as_str())
    );

    // attempt_count == min_attempts - 1 → no short-circuit, but the record is
    // still reported so a successful retrieval can drop it.
    let below = lookup_absence(query, &store, 3, NS, REVISION).await;
    assert!(!below.short_circuit);
    assert_eq!(below.entry_id.as_deref(), Some(query_id(query).as_str()));
}

#[tokio::test]
async fn lookup_absence_ignores_stale_revision() {
    let query = "query recorded before the content changed";
    let store = store_with(entry(query, 5, NS, REVISION));

    // Same namespace, newer revision: the record described older content and
    // must not suppress retrieval — but it is still returned for invalidation.
    let stale = lookup_absence(query, &store, 3, NS, REVISION + 1).await;
    assert!(
        !stale.short_circuit,
        "a record from an older revision must not short-circuit"
    );
    assert!(
        stale.entry_id.is_some(),
        "a stale record must still be reported so it can be deleted"
    );

    // The revision it was recorded at still short-circuits.
    assert!(
        lookup_absence(query, &store, 3, NS, REVISION)
            .await
            .short_circuit
    );
}

#[tokio::test]
async fn lookup_absence_ignores_other_namespaces() {
    let query = "query absent in another namespace";
    let store = store_with(entry(query, 9, "other-ns", REVISION));

    let foreign = lookup_absence(query, &store, 1, NS, REVISION).await;
    assert!(
        !foreign.short_circuit,
        "another namespace's record must not suppress this namespace"
    );
    assert!(
        foreign.entry_id.is_none(),
        "another namespace's record must not be deleted by this namespace"
    );
}

fn query_id(query: &str) -> String {
    AbsenceEntry::id_for(query)
}
