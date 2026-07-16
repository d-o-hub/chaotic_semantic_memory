//! Absence short-circuit threshold for hybrid BM25 (ADR-0094 / Wave 32).

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::bridge_persistence::{AbsenceEntry, AbsenceStore};

/// Default BM25 absence short-circuit threshold. Override: `CSM_ABSENCE_MIN_ATTEMPTS`.
pub const DEFAULT_ABSENCE_MIN_ATTEMPTS: u32 = 3;

/// Env `CSM_ABSENCE_MIN_ATTEMPTS` if >0, else [`DEFAULT_ABSENCE_MIN_ATTEMPTS`].
#[must_use]
pub fn absence_min_attempts() -> u32 {
    std::env::var("CSM_ABSENCE_MIN_ATTEMPTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_ABSENCE_MIN_ATTEMPTS)
}

/// True when absence memory has attempt_count >= min_attempts (CLI hybrid skips BM25).
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub async fn is_known_absent(query: &str, store: &dyn AbsenceStore, min_attempts: u32) -> bool {
    match store.get_absence(&AbsenceEntry::id_for(query)).await {
        Ok(Some(entry)) => entry.attempt_count >= min_attempts,
        _ => false,
    }
}
