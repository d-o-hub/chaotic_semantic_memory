//! `operator_shutdown` must wait for a signal rather than resolve on its own.
//!
//! This is the unit-level half of ADR-0099's guarantee, and it exists because the
//! mutation gate cannot see the integration test: `scripts/mutation_test.sh` runs
//! the fast profile as `cargo mutants --lib -p csm-retrieval -p
//! chaotic_semantic_memory` (`scripts/mutation_test.sh:142`), so `tests/**` never
//! executes. Measured on CI run 37276860430 (head `0107c9c`): all three in-diff
//! mutants in this file survived at score 0.0% —
//!
//! ```text
//! MISSED src/shutdown.rs:21:5: replace operator_shutdown with ()
//! MISSED src/shutdown.rs:40:5: replace sigint with ()
//! MISSED src/shutdown.rs:56:5: replace sigterm with ()
//! ```
//!
//! Each arm is asserted separately so a failure names the arm that broke rather
//! than only the composite. The property is the one that was actually wrong here:
//! a missing SIGTERM registration does not resolve early, it silently leaves the
//! default disposition in place — so the observable a unit test can reach is
//! "no signal, no resolution", while the positive direction (a real SIGTERM stops
//! the process with exit 0, not 143) needs a process exit status and lives in
//! `tests/cli_shutdown_signal.rs`.

use std::time::Duration;

/// Long enough for the correct code to prove it is parked, short enough to stay
/// out of the mutation budget. A mutant that resolves instantly is observed on the
/// first poll, so this bound cannot make the assertion flaky in the passing
/// direction — the correct future never becomes ready.
const NO_SIGNAL: Duration = Duration::from_millis(100);

#[tokio::test]
async fn operator_shutdown_waits_for_a_signal() {
    assert!(
        tokio::time::timeout(NO_SIGNAL, super::operator_shutdown())
            .await
            .is_err(),
        "`operator_shutdown` resolved with no signal sent; every caller \
         (`csm watch`, `csm mcp serve`) would exit the moment it started"
    );
}

#[tokio::test]
async fn sigint_arm_waits_for_a_signal() {
    assert!(
        tokio::time::timeout(NO_SIGNAL, super::sigint())
            .await
            .is_err(),
        "the SIGINT arm resolved with no signal sent, so `select!` would report a \
         shutdown that never happened"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn sigterm_arm_waits_for_a_signal() {
    assert!(
        tokio::time::timeout(NO_SIGNAL, super::sigterm())
            .await
            .is_err(),
        "the SIGTERM arm resolved with no signal sent — the same failure that made a \
         service-managed `csm mcp serve` die on the default disposition instead of \
         running the bounded TTL-cleanup stop"
    );
}
