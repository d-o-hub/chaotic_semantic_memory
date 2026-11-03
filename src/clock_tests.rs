//! Unit tests for the shared clock (issue #952).
//!
//! The clock exists so the `cfg(target_arch = "wasm32")` selection has no
//! arithmetic the native mutation profile cannot compile. These tests are in the
//! lib target, which is the only target `scripts/mutation_test.sh` runs.

use super::Clock;
use std::time::Duration;

#[test]
fn a_fresh_clock_does_not_report_negative_time() {
    let elapsed = Clock::now().elapsed_ms();
    assert!(
        elapsed <= 5,
        "a fresh read should be near zero, got {elapsed}"
    );
}

#[test]
fn elapsed_advances_after_a_sleep() {
    let start = Clock::now();
    std::thread::sleep(Duration::from_millis(5));
    let elapsed = start.elapsed_ms();
    assert!(
        elapsed >= 4,
        "expected at least 4 ms after a 5 ms sleep, got {elapsed}"
    );
}

#[test]
fn repeated_reads_from_one_start_are_monotonic() {
    let start = Clock::now();
    let first = start.elapsed_ms();
    std::thread::sleep(Duration::from_millis(2));
    let second = start.elapsed_ms();
    assert!(
        second >= first,
        "clock moved backwards: {first} then {second}"
    );
}
