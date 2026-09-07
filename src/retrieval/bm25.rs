//! BM25 keyword search index for hybrid retrieval.
//! Implements Okapi BM25 for exact keyword matching.

pub use csm_retrieval::{Bm25Config, Bm25Index};

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use csm_traits::{AbsenceEntry, AbsenceStore};

/// Returns true if the query has a known absence record with
/// attempt_count >= min_attempts, indicating BM25 should be skipped.
/// Called by the framework probe paths to skip retrieval for queries that
/// repeatedly abstained.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn is_known_absent(query: &str, store: &dyn AbsenceStore, min_attempts: u32) -> bool {
    let id = AbsenceEntry::id_for(query);
    match store.get_absence(&id).await {
        Ok(Some(entry)) => entry.attempt_count >= min_attempts,
        _ => false,
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
#[path = "bm25/absence_short_circuit_tests.rs"]
mod absence_short_circuit_tests;
