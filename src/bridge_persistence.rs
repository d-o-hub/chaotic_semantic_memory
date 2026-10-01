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
/// owner-neutral persistence contract, stamped with the namespace and revision
/// the query was observed at (ADR-0093).
pub fn absence_from_abstention(
    abstention: &RetrievalAbstention,
    namespace: &str,
    namespace_revision: u64,
) -> AbsenceEntry {
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
        namespace: namespace.to_string(),
        namespace_revision,
    }
}

/// Merge a new abstention event into an existing entry (upsert logic).
///
/// An absence record is only authoritative for the content it was observed
/// against: when the namespace or the namespace revision changed, the earlier
/// attempts described content that no longer exists, so the counter restarts at
/// one instead of accumulating attempts across content changes.
pub fn merge_absence_with(
    entry: &mut AbsenceEntry,
    abstention: &RetrievalAbstention,
    namespace: &str,
    namespace_revision: u64,
) {
    if entry.namespace != namespace || entry.namespace_revision != namespace_revision {
        entry.namespace = namespace.to_string();
        entry.namespace_revision = namespace_revision;
        entry.attempt_count = 1;
    } else {
        entry.attempt_count += 1;
    }
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
    namespace: &str,
    namespace_revision: u64,
) -> Result<AbsenceEntry> {
    let id = AbsenceEntry::id_for(&abstention.query);
    match store.get_absence(&id).await? {
        Some(mut existing) => {
            merge_absence_with(&mut existing, abstention, namespace, namespace_revision);
            store.upsert_absence(&existing).await?;
            Ok(existing)
        }
        None => {
            let entry = absence_from_abstention(abstention, namespace, namespace_revision);
            store.upsert_absence(&entry).await?;
            Ok(entry)
        }
    }
}
