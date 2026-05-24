//! CLI ↔ Framework parity smoke test (ADR-0066, Wave 21 P0)
//!
//! Verifies the binary exposes every subcommand promised by ADR-0066 so a
//! stale install or accidental wiring regression cannot silently drop a
//! command. Each entry below maps to a public `Framework` API surface.
//!
//! Run with: `cargo test --test cli_parity --features cli`

#![cfg(feature = "cli")]

use std::process::Command;

const EXPECTED_SUBCOMMANDS: &[&str] = &[
    // Core lifecycle
    "inject",
    "probe",
    "query",
    "associate",
    "export",
    "import",
    "version",
    "completions",
    // Indexing
    "index-jsonl",
    "index-dir",
    // Wave 21 P0 — ADR-0066 parity additions
    "delete",
    "get",
    "update",
    "disassociate",
    "associations",
    "traverse",
    "path",
    "probe-filtered",
    "stats",
    "metrics",
    "watch",
    "probe-graph",
    "history",
    "diff",
    "rollback",
];

fn csm_bin() -> std::path::PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_csm"))
}

#[test]
fn cli_help_lists_every_expected_subcommand() {
    let output = Command::new(csm_bin())
        .arg("--help")
        .output()
        .expect("failed to spawn csm --help");
    assert!(output.status.success(), "csm --help exited non-zero");

    let stdout = String::from_utf8(output.stdout).expect("csm --help non-UTF8");

    let mut missing = Vec::new();
    for cmd in EXPECTED_SUBCOMMANDS {
        // clap renders entries as `  <name>` (two-space indent) followed by
        // either whitespace or end-of-line. Match the indented form so we
        // don't false-positive against descriptions.
        let needle = format!("  {cmd} ");
        let needle_eol = format!("  {cmd}\n");
        if !stdout.contains(&needle) && !stdout.contains(&needle_eol) {
            missing.push(*cmd);
        }
    }

    assert!(
        missing.is_empty(),
        "csm --help is missing subcommands required by ADR-0066: {missing:?}\nFull help output:\n{stdout}"
    );
}

#[test]
fn cli_each_subcommand_has_help() {
    for cmd in EXPECTED_SUBCOMMANDS {
        let output = Command::new(csm_bin())
            .args([cmd, "--help"])
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn csm {cmd} --help: {e}"));

        assert!(
            output.status.success(),
            "csm {cmd} --help exited non-zero: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
