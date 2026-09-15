//! Persistence for canonical concept graph.
//!
//! Feature-gated persistence layer for storing the symbolic semantic graph
//! used by the bridge retrieval pipeline.
//!
//! The durable CRUD itself lives in `csm-persistence` (`persistence_bridge.rs`,
//! ADR-0094) and is available on `Persistence` through the root re-export. What
//! remains here is the root-only adapter between the framework's abstention
//! event and the owner-neutral `csm_traits::AbsenceStore` contract.

// Casts are intentional for version serialization

use crate::retrieval::hybrid::RetrievalAbstention;
use csm_core_lib::error::Result;
use csm_traits::{AbsenceEntry, AbsenceStore};

/// Build an `AbsenceEntry` from a `RetrievalAbstention` event.
///
/// Root adapter: `AbsenceEntry`/`AbsenceStore` live in `csm-traits` (ADR-0094);
/// this conversion bridges the framework-level abstention event into the
/// owner-neutral persistence contract.
pub fn absence_from_abstention(abstention: &RetrievalAbstention) -> AbsenceEntry {
    let normalized = AbsenceEntry::normalize(&abstention.query);
    AbsenceEntry {
        id: AbsenceEntry::id_for(&abstention.query),
        query: abstention.query.clone(),
        normalized_query: normalized,
        attempt_count: 1,
        last_threshold: abstention.min_score_threshold,
        best_score_ever: abstention.best_score_seen,
        first_seen: abstention.timestamp,
        last_seen: abstention.timestamp,
    }
}

/// Merge a new abstention event into an existing entry (upsert logic).
pub fn merge_absence_with(entry: &mut AbsenceEntry, abstention: &RetrievalAbstention) {
    entry.attempt_count += 1;
    entry.last_seen = abstention.timestamp;
    entry.last_threshold = abstention.min_score_threshold;

    match (abstention.best_score_seen, entry.best_score_ever) {
        (Some(new), Some(existing)) => {
            if new > existing {
                entry.best_score_ever = Some(new);
            }
        }
        (Some(new), None) => {
            entry.best_score_ever = Some(new);
        }
        _ => {}
    }
}

/// Persist a RetrievalAbstention event as an AbsenceEntry.
pub async fn persist_absence(
    abstention: &RetrievalAbstention,
    store: &dyn AbsenceStore,
) -> Result<AbsenceEntry> {
    let id = AbsenceEntry::id_for(&abstention.query);
    match store.get_absence(&id).await? {
        Some(mut existing) => {
            merge_absence_with(&mut existing, abstention);
            store.upsert_absence(&existing).await?;
            Ok(existing)
        }
        None => {
            let entry = absence_from_abstention(abstention);
            store.upsert_absence(&entry).await?;
            Ok(entry)
        }
    }
}
