//! BM25 keyword search index for hybrid retrieval.
//! Implements Okapi BM25 for exact keyword matching.

pub use csm_retrieval::{Bm25Config, Bm25Index};

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use csm_traits::{AbsenceEntry, AbsenceStore};

/// Result of consulting the persisted absence records for a query.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
#[derive(Debug, Clone)]
pub struct AbsenceLookup {
    /// Entry id when a record exists for this namespace (invalidation target
    /// once the query retrieves successfully).
    pub entry_id: Option<String>,
    /// True when the record is authoritative for the current content and the
    /// attempt threshold is reached, i.e. retrieval may be skipped.
    pub short_circuit: bool,
}

/// Consult the persisted absence records for an unfiltered semantic query.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn lookup_absence(
    query: &str,
    store: &dyn AbsenceStore,
    min_attempts: u32,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceLookup {
    lookup_absence_by_id(
        AbsenceEntry::id_for(query),
        store,
        min_attempts,
        namespace,
        namespace_revision,
    )
    .await
}

/// Consult an absence record using a stable retrieval-scope key and bounded TTL.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn lookup_absence_scoped(
    query: &str,
    scope: &str,
    store: &dyn AbsenceStore,
    min_attempts: u32,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceLookup {
    lookup_absence_by_id(
        AbsenceEntry::id_for_scoped(scope, query),
        store,
        min_attempts,
        namespace,
        namespace_revision,
    )
    .await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
async fn lookup_absence_by_id(
    id: String,
    store: &dyn AbsenceStore,
    min_attempts: u32,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceLookup {
    const TTL_SECS: i64 = 300;
    match store.get_absence(&id).await {
        Ok(Some(entry)) if entry.namespace == namespace => {
            let age = chrono::Utc::now().signed_duration_since(entry.last_seen);
            AbsenceLookup {
                short_circuit: entry.attempt_count >= min_attempts
                    && entry.namespace_revision == namespace_revision
                    && age >= chrono::Duration::zero()
                    && age.num_seconds() < TTL_SECS,
                entry_id: Some(id),
            }
        }
        _ => AbsenceLookup {
            entry_id: None,
            short_circuit: false,
        },
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
#[path = "bm25/absence_short_circuit_tests.rs"]
mod absence_short_circuit_tests;
