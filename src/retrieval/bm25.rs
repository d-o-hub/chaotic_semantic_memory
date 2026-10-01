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

/// Consult the absence records for a query in one read.
///
/// A record only short-circuits when it belongs to this namespace **and** was
/// recorded at the namespace's current revision: any content mutation bumps the
/// revision (ADR-0093), which makes earlier absence observations stale — the
/// store may now hold content that matches the query. `entry_id` is returned
/// even for a stale or below-threshold record so a successful retrieval can
/// drop it.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn lookup_absence(
    query: &str,
    store: &dyn AbsenceStore,
    min_attempts: u32,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceLookup {
    let id = AbsenceEntry::id_for(query);
    match store.get_absence(&id).await {
        Ok(Some(entry)) if entry.namespace == namespace => AbsenceLookup {
            short_circuit: entry.attempt_count >= min_attempts
                && entry.namespace_revision == namespace_revision,
            entry_id: Some(id),
        },
        _ => AbsenceLookup {
            entry_id: None,
            short_circuit: false,
        },
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
#[path = "bm25/absence_short_circuit_tests.rs"]
mod absence_short_circuit_tests;
