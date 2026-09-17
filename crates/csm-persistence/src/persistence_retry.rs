//! Bounded retries and lock timeouts for local SQLite writes (ADR-0095).
//!
//! The ADR requires persistence concurrency to have *bounded* retries and
//! timeouts and to report p50/p95/p99, throughput, retry and error rates. The
//! 2026-09-17 scale evidence measured 90 % failures with eight concurrent
//! writers before this module existed
//! (`plans/evidence/scale_2026_09_17/persistence_scale.json`).
//!
//! Two bounds apply:
//!
//! - [`LOCAL_BUSY_TIMEOUT_MS`] — each local connection sets
//!   `PRAGMA busy_timeout`, so SQLite *waits* for a competing writer instead
//!   of failing immediately.
//! - [`WRITE_RETRY_LIMIT`] — idempotent writes (upserts in a transaction)
//!   retry a transient lock error with an exponential backoff capped by
//!   [`backoff`], so a writer that still loses a race recovers instead of
//!   surfacing `database is locked` to the caller.
//!
//! Retries are deliberately limited to idempotent write paths: re-running an
//! upsert transaction is safe, re-running an arbitrary statement is not.

use std::time::Duration;

use csm_core_lib::error::MemoryError;

/// Bounded wait applied per connection via `PRAGMA busy_timeout`.
pub(crate) const LOCAL_BUSY_TIMEOUT_MS: u64 = 5_000;

/// Bounded number of retries for a transient failure of an idempotent write.
pub(crate) const WRITE_RETRY_LIMIT: u32 = 5;

/// A transient contention failure, as opposed to a schema or constraint error.
///
/// libSQL surfaces SQLite's `SQLITE_BUSY`/`SQLITE_LOCKED` families through the
/// error text; the classification stays textual because the driver does not
/// expose the extended result code.
pub(crate) fn is_transient(error: &MemoryError) -> bool {
    let text = format!("{error:?}").to_lowercase();
    text.contains("locked") || text.contains("busy")
}

/// Deterministic exponential backoff for retry `attempt` (1-based):
/// 2, 4, 8, 16, 32 ms — no jitter, so evidence runs stay reproducible.
pub(crate) fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(2u64 << attempt.saturating_sub(1).min(5))
}
