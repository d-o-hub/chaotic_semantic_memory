//! Deterministic CI path selection for `chaotic_semantic_memory`.
//!
//! `ci.yml`'s `detect-changes` job decides which specialized jobs a run needs.
//! That decision used to be a block of shell regexes embedded in the workflow
//! (issue #992): untested, and wrong for the MCP family — `src/cli/mcp.rs` and
//! `tests/mcp*_integration.rs` did not match the `mcp` rule, so a pull request
//! could change MCP integration and skip the dedicated `cargo test --features
//! mcp` job while its diff looked covered.
//!
//! This crate owns that decision instead:
//!
//! * [`policy`] declares, as path globs, which change selects which job — and
//!   *why*, in the same table the tests read.
//! * [`changed`] resolves the change set from a committed range (`git
//!   merge-base` + `git diff --name-status -z`) or from a NUL-separated path
//!   list, keeping Git diagnostics instead of turning them into an empty set.
//! * [`selection`] maps the change set onto the jobs, deterministically.
//!
//! The contract, in one paragraph: **a job is skipped only when every changed
//! path is proven irrelevant to it.** Discovery that fails for any reason
//! (unresolvable ref, no merge base, unreadable Git, non-UTF-8 path, a shallow
//! clone missing the object) is not evidence of an empty diff, so it selects
//! every job with the diagnostic attached. An empty change set is "no
//! information" and selects every job too. A path that matches no rule at all
//! is unclassified and selects every job. Only documentation, plans, skills and
//! tooling — declared in [`policy::NON_BUILD_PATHS`] — select nothing. The
//! same change set always yields the same split with the same reasons.
//!
//! Selection is deliberately conservative at *job* granularity: every
//! specialized job compiles workspace crates (the root crate and its
//! dependencies, or `csm-duckdb`/`csm-wasm` and theirs), and a path alone
//! cannot prove a crate is unrelated. So any Rust, manifest, lockfile,
//! build-script, toolchain or CI-workflow change selects the specialized jobs;
//! the selection's value is that a docs-only or tooling-only pull request skips
//! them entirely, and that the decision has one owner, tests and reasons.
//!
//! Consumed by `.github/workflows/ci.yml` through the `csm-ci-select` binary:
//!
//! ```text
//! cargo run --locked --quiet -p csm-ci-selection -- \
//!     --since origin/main --github-output
//! ```
//!
//! The workflow wraps that invocation so that *any* failure — including the
//! selector not building — enables every job rather than skipping one.

pub mod changed;
pub mod policy;
pub mod selection;

pub use changed::{
    ChangeKind, ChangedFile, ChangedFiles, Discovery, GitRunner, ProcessGit, ResolvedRange,
    discover_since, parse_name_status,
};
pub use policy::{
    JOBS, Job, JobMatch, JobSpec, NON_BUILD_PATHS, PathPattern, Policy, PolicyError,
    SHARED_BUILD_INPUTS, TrunkPolicy,
};
pub use selection::{
    ALL_JOBS_REASON, FAIL_CLOSED_REASON, Mode, NO_CHANGE_INFORMATION_REASON, NO_MATCH_REASON,
    Selected, Selection, Skipped, TRUNK_ALWAYS_REASON, TRUNK_NEVER_REASON, UNCLASSIFIED_REASON,
    select, select_all,
};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    mod changed_tests {
        use super::*;
        include!("changed_tests.rs");
    }
    mod policy_tests {
        use super::*;
        include!("policy_tests.rs");
    }
    mod selection_tests {
        use super::*;
        include!("selection_tests.rs");
    }
}
