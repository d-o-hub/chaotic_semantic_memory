//! Monotonic millisecond clock with one `cfg`-split source.
//!
//! Issue #952: the persistence-latency sites used a `cfg` pair — native
//! `Instant::now()` / `elapsed()`, wasm `Date::now()` and `Date::now() - start`
//! — inside production functions. `cargo-mutants` parses source text, so the
//! wasm-only subtraction produced two mutants that the native `--lib` profile
//! never compiles: they build to the same program and pass the same tests, and
//! were reported MISSED forever. Here the arithmetic site is native-compiled and
//! `cfg` selects only the millisecond source, so a new clock mutant is killable
//! instead of invisible.
//!
//! The platform source lives in a child file rather than as two same-named
//! functions in this module: `scripts/check_cfg_mutants.py` flags same-file
//! cfg-split declarations, and a `mod` pair carries no arithmetic mutants.

#[cfg(not(target_arch = "wasm32"))]
#[path = "clock_native.rs"]
mod imp;

#[cfg(target_arch = "wasm32")]
#[path = "clock_wasm.rs"]
mod imp;

/// A point on the process clock.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock(u64);

impl Clock {
    /// Capture the current monotonic millisecond value.
    #[must_use]
    pub(crate) fn now() -> Self {
        Self(now_ms())
    }

    /// Milliseconds elapsed since this value was captured.
    ///
    /// `saturating_sub` keeps the answer non-negative if the platform clock is
    /// adjusted backwards between the two reads.
    #[must_use]
    pub(crate) fn elapsed_ms(self) -> u64 {
        now_ms().saturating_sub(self.0)
    }
}

fn now_ms() -> u64 {
    imp::now_ms()
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
