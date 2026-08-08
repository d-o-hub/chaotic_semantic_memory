//! Absence-memory read path and probe short-circuit helpers.
//!
//! Complements write-side [`crate::bridge_persistence::persist_absence`]: after
//! enough failed retrievals for a normalized query, probes can skip re-search
//! and return an immediate abstention. Short-circuit is fail-open on store
//! errors and disabled when `min_attempts == 0`.

use crate::bridge_persistence::{AbsenceEntry, AbsenceStore};
use crate::retrieval::hybrid::{HybridResult, RetrievalAbstention};

/// Default failed-attempt threshold before short-circuiting probes.
///
/// A single empty-store probe must not permanently suppress search; operators
/// listing gaps via MCP can still use `min_attempts = 1`.
pub const DEFAULT_ABSENCE_SHORT_CIRCUIT_MIN_ATTEMPTS: u32 = 3;

/// Mode marker recorded on short-circuit abstentions.
pub const KNOWN_ABSENT_MODE: &str = "KnownAbsent";

/// Load the absence entry when `attempt_count >= min_attempts`.
///
/// Returns `None` when short-circuit is disabled (`min_attempts == 0`), the
/// entry is missing, the threshold is not met, or the store fails (fail-open).
/// The namespace is folded into the absence ID, so entries are per-namespace.
pub async fn known_absence_entry(
    ns: &str,
    query: &str,
    store: &dyn AbsenceStore,
    min_attempts: u32,
) -> Option<AbsenceEntry> {
    if min_attempts == 0 {
        return None;
    }
    let id = AbsenceEntry::id_for(ns, query);
    match store.get_absence(&id).await {
        Ok(Some(entry)) if entry.attempt_count >= min_attempts => Some(entry),
        _ => None,
    }
}

/// Returns true if the query has a known absence record at or above threshold.
pub async fn is_known_absent(
    ns: &str,
    query: &str,
    store: &dyn AbsenceStore,
    min_attempts: u32,
) -> bool {
    known_absence_entry(ns, query, store, min_attempts)
        .await
        .is_some()
}

/// Build an abstention result for a known-absent short-circuit.
///
/// Does not persist or increment `attempt_count` — re-reads are not new evidence.
#[must_use]
pub fn short_circuit_abstention(query: &str, entry: &AbsenceEntry) -> HybridResult {
    HybridResult::Abstained(RetrievalAbstention {
        query: query.to_string(),
        min_score_threshold: entry.last_threshold,
        best_score_seen: entry.best_score_ever,
        attempted_modes: vec![KNOWN_ABSENT_MODE.to_string()],
        timestamp: chrono::Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::persistence::Persistence;
    use crate::retrieval::hybrid::RetrievalAbstention;
    use chrono::Utc;
    use tempfile::NamedTempFile;

    async fn store_with_attempts(ns: &str, query: &str, attempts: u32) -> (NamedTempFile, Persistence) {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();
        let persistence = Persistence::new_local(path).await.unwrap();
        for i in 0..attempts {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: 0.5,
                best_score_seen: Some(0.1 * (i as f32 + 1.0)),
                attempted_modes: vec!["Auto".to_string()],
                timestamp: Utc::now(),
            };
            crate::bridge_persistence::persist_absence(ns, &abstention, &persistence)
                .await
                .unwrap();
        }
        (temp, persistence)
    }

    #[tokio::test]
    async fn threshold_not_met_returns_none() {
        let (_temp, store) = store_with_attempts("_default", "missing topic", 2).await;
        assert!(!is_known_absent("_default", "missing topic", &store, 3).await);
        assert!(known_absence_entry("_default", "missing topic", &store, 3)
            .await
            .is_none());
    }

    #[tokio::test]
    async fn threshold_met_returns_entry() {
        let (_temp, store) = store_with_attempts("_default", "missing topic", 3).await;
        assert!(is_known_absent("_default", "missing topic", &store, 3).await);
        let entry = known_absence_entry("_default", "missing topic", &store, 3)
            .await
            .expect("entry");
        assert_eq!(entry.attempt_count, 3);
    }

    #[tokio::test]
    async fn min_attempts_zero_disables_short_circuit() {
        let (_temp, store) = store_with_attempts("_default", "disabled", 5).await;
        assert!(!is_known_absent("_default", "disabled", &store, 0).await);
    }

    #[tokio::test]
    async fn normalization_matches_padded_query() {
        let (_temp, store) = store_with_attempts("_default", "  Foo Bar  ", 3).await;
        assert!(is_known_absent("_default", "foo bar", &store, 3).await);
    }

    #[tokio::test]
    async fn absence_is_scoped_per_namespace() {
        let (_temp, store) = store_with_attempts("tenant-a", "scoped query", 3).await;
        // Same text in a different namespace must not short-circuit.
        assert!(!is_known_absent("tenant-b", "scoped query", &store, 3).await);
        assert!(known_absence_entry("tenant-b", "scoped query", &store, 3)
            .await
            .is_none());
        // Original namespace still short-circuits.
        assert!(is_known_absent("tenant-a", "scoped query", &store, 3).await);
    }

    #[tokio::test]
    async fn short_circuit_result_marks_known_absent() {
        let entry = AbsenceEntry {
            id: AbsenceEntry::id_for("_default", "q"),
            query: "q".to_string(),
            normalized_query: "q".to_string(),
            attempt_count: 3,
            last_threshold: 0.7,
            best_score_ever: Some(0.2),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
        };
        match short_circuit_abstention("q", &entry) {
            HybridResult::Abstained(a) => {
                assert_eq!(a.attempted_modes, vec![KNOWN_ABSENT_MODE.to_string()]);
                assert!((a.min_score_threshold - 0.7).abs() < f32::EPSILON);
                assert!((a.best_score_seen.unwrap() - 0.2).abs() < f32::EPSILON);
            }
            HybridResult::Success(_) => panic!("expected abstention"),
        }
    }

    #[tokio::test]
    async fn clear_absences_removes_short_circuit() {
        let (_temp, store) = store_with_attempts("_default", "clear me", 3).await;
        assert!(is_known_absent("_default", "clear me", &store, 3).await);
        store.clear_absences().await.unwrap();
        assert!(!is_known_absent("_default", "clear me", &store, 3).await);
    }
}
