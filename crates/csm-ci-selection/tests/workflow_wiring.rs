//! The `detect-changes` job must stay wired to this selector.
//!
//! `ci.yml` publishes the selector's outputs and gates jobs on them; these
//! assertions fail if a key disappears from the workflow, is published but gates
//! nothing, or if the classification drifts back into inline shell regexes. The
//! selector's *behaviour* is proven in `selection_acceptance.rs`; this file
//! proves the wiring that carries it into CI.

use std::path::{Path, PathBuf};

/// Source of truth for the job outputs, and the workflow text they live in.
///
/// `#[cfg(test)]` is what `.clippy.toml`'s `allow-expect-in-tests` recognises as
/// test code, so an unreadable `ci.yml` fails the test instead of the lint.
#[cfg(test)]
mod harness {
    use super::*;

    /// The job outputs the selector owns, from the single table in the library:
    /// a job added to `JOBS` is asserted here without editing a copy.
    pub(super) fn job_names() -> Vec<&'static str> {
        csm_ci_selection::policy::JOBS
            .iter()
            .map(|spec| spec.name)
            .collect()
    }

    pub(super) fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository root")
    }

    pub(super) fn read_workflow() -> String {
        std::fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("ci.yml")
    }

    /// The job's block: from `  <name>:` to the next two-space job key.
    pub(super) fn job_block(workflow: &str, name: &str) -> String {
        let header = format!("  {name}:");
        let mut block = String::new();
        let mut inside = false;
        for line in workflow.lines() {
            if line.starts_with(&header) {
                inside = true;
                block.push_str(line);
                block.push('\n');
                continue;
            }
            if inside {
                if line.starts_with("  ")
                    && !line.starts_with("   ")
                    && line.trim_end().ends_with(':')
                {
                    break;
                }
                block.push_str(line);
                block.push('\n');
            }
        }
        assert!(!block.is_empty(), "job `{name}` not found in ci.yml");
        block
    }
}

use harness::*;

#[test]
fn the_workflow_consumes_this_selector_and_keeps_no_inline_classification() {
    let workflow = read_workflow();
    for name in job_names() {
        assert!(
            workflow.contains(&format!(
                "      {name}: ${{{{ steps.filter.outputs.{name} }}}}"
            )),
            "detect-changes must publish the `{name}` output"
        );
        assert!(
            workflow.contains(&format!("needs.detect-changes.outputs.{name}")),
            "the `{name}` output must gate a job, or it is dead configuration"
        );
    }

    let block = job_block(&workflow, "detect-changes");
    assert!(
        block.contains("csm-ci-selection") && block.contains("--github-output"),
        "detect-changes must call the selector: {block}"
    );
    for leftover in ["grep -qE", "match '", "Pattern="] {
        assert!(
            !block.contains(leftover),
            "`{leftover}` is inline path classification the selector now owns"
        );
    }
    assert!(
        block.contains("||"),
        "the selector invocation needs a fallback that runs every job when it cannot classify"
    );
}
