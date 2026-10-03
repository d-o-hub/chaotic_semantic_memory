#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! CLI ↔ Framework parity smoke test (ADR-0066, Wave 21 P0)
//!
//! Verifies the binary exposes every subcommand promised by ADR-0066 so a
//! stale install or accidental wiring regression cannot silently drop a
//! command. Each entry below maps to a public `Framework` API surface.
//!
//! `cli_each_subcommand_has_help` is the third check: help coverage is derived
//! from the real `Commands` enum through clap itself (not from
//! `EXPECTED_SUBCOMMANDS`), so a subcommand that exists but has never had its
//! `--help` rendered fails here even when nobody added it to the hand-maintained
//! list — that list is still checked, against the derived set, so the two cannot
//! drift silently.
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
fn cli_history_flags_listed_in_help() {
    let output = Command::new(csm_bin())
        .args(["history", "--help"])
        .output()
        .expect("failed to spawn csm history --help");
    assert!(
        output.status.success(),
        "csm history --help exited non-zero"
    );
    let stdout = String::from_utf8(output.stdout).expect("csm history --help non-UTF8");
    for flag in &["--version", "--rollback", "--confirm"] {
        assert!(
            stdout.contains(flag),
            "csm history --help should mention {flag}, got:\n{stdout}"
        );
    }
}

// ── Help coverage derived from clap, not from a hand-maintained list ───────
//
// `EXPECTED_SUBCOMMANDS` can only ever re-check the list someone already
// maintains. These helpers read the command tree out of the real `Commands`
// enum (`CliArgs::command()`), so "a subcommand exists but its help was never
// rendered" is caught even when nobody remembered to extend the const above.

/// Subcommands clap generates itself rather than deriving from a `Commands`
/// variant. `csm help --help` is rejected by clap's generated `help` command
/// (`error: unrecognized subcommand '--help'`, exit 2) because its positional
/// takes command names, not flags — so it is not a `Commands` variant and is
/// excluded from coverage. Nothing else in the tree is generated.
const CLAP_GENERATED: &[&str] = &["help"];

/// Collect every reachable subcommand path (e.g. `["mcp", "serve"]`), depth
/// first. Nested groups are included so a variant that carries its own
/// subcommands cannot hide behind a group-level help screen.
fn collect_subcommand_paths(cmd: &clap::Command, path: &[String], out: &mut Vec<Vec<String>>) {
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        if CLAP_GENERATED.contains(&name) {
            continue;
        }
        let mut next = path.to_vec();
        next.push(name.to_string());
        out.push(next.clone());
        collect_subcommand_paths(sub, &next, out);
    }
}

/// Resolve a path back to its `clap::Command` definition.
fn command_at<'a>(root: &'a clap::Command, path: &[String]) -> Option<&'a clap::Command> {
    let mut current = root;
    for name in path {
        current = current.find_subcommand(name)?;
    }
    Some(current)
}

/// Long flags and positional placeholders the definition of `cmd` declares.
/// `--help` must render all of them: that is what "long help" means here — the
/// `--help` screen is the one that lists every argument, and an empty or
/// truncated screen drops tokens.
///
/// Blind spot kept honest: clap 4.6 exposes no public `Arg::is_hidden()`, and
/// `hide` is used nowhere in `src/cli/`, so every declared argument is expected
/// in the help text. If a hidden flag is ever added deliberately, this helper is
/// where the exemption belongs — said out loud, not silently passing.
fn required_help_tokens(cmd: &clap::Command) -> Vec<String> {
    let mut tokens = Vec::new();
    for arg in cmd.get_arguments() {
        if let Some(long) = arg.get_long() {
            tokens.push(format!("--{long}"));
        } else if arg.is_positional() {
            // clap renders a required positional as `<NAME>` and an optional one
            // as `[NAME]` (multi-value ones as `[NAME]...`, still a substring).
            let (open, close) = if arg.is_required_set() {
                ("<", ">")
            } else {
                ("[", "]")
            };
            tokens.push(format!(
                "{open}{}{close}",
                arg.get_id().as_str().to_uppercase()
            ));
        } else if let Some(short) = arg.get_short() {
            tokens.push(format!("-{short}"));
        }
    }
    tokens
}

#[test]
fn cli_each_subcommand_has_help() {
    use clap::CommandFactory;

    let root = <chaotic_semantic_memory::cli::CliArgs as CommandFactory>::command();
    let mut paths = Vec::new();
    collect_subcommand_paths(&root, &[], &mut paths);

    // Deriving is worthless if it can silently derive nothing.
    assert!(
        paths.len() >= EXPECTED_SUBCOMMANDS.len(),
        "clap derived {paths:?} — fewer commands than EXPECTED_SUBCOMMANDS \
         ({}), so the derivation itself is broken",
        EXPECTED_SUBCOMMANDS.len()
    );

    // The derived top-level set must equal the hand-maintained list (plus the
    // `mcp` variant, which only exists when the feature is on), or the two
    // drift apart and this file starts testing different things.
    let mut derived: Vec<&str> = paths
        .iter()
        .filter(|p| p.len() == 1)
        .map(|p| p[0].as_str())
        .collect();
    derived.sort_unstable();
    let mut expected: Vec<&str> = EXPECTED_SUBCOMMANDS.to_vec();
    #[cfg(feature = "mcp")]
    expected.push("mcp");
    expected.sort_unstable();
    assert_eq!(
        derived, expected,
        "clap's Commands enum and EXPECTED_SUBCOMMANDS disagree \
         (clap is the source of truth for coverage; the const is the ADR-0066 promise)"
    );

    for path in &paths {
        let printable = path.join(" ");
        let cmd = command_at(&root, path)
            .unwrap_or_else(|| panic!("derived path {path:?} is not resolvable in the tree"));

        let output = Command::new(csm_bin())
            .args(
                path.iter()
                    .map(String::as_str)
                    .chain(std::iter::once("--help")),
            )
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn csm {printable} --help: {e}"));

        assert!(
            output.status.success(),
            "csm {printable} --help exited non-zero: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.is_empty(),
            "csm {printable} --help wrote to stderr (help belongs on stdout): {stderr}"
        );

        let stdout = String::from_utf8(output.stdout)
            .unwrap_or_else(|e| panic!("csm {printable} --help non-UTF8: {e}"));

        // A real help screen, for *this* subcommand — not a bare usage line,
        // not another command's help, and not clap's error hint.
        assert!(
            stdout.contains(&format!("Usage: csm {printable}")),
            "csm {printable} --help does not render its own usage line:\n{stdout}"
        );
        assert!(
            stdout.contains("Options:"),
            "csm {printable} --help rendered no Options section:\n{stdout}"
        );
        assert!(
            stdout.contains("-h, --help"),
            "csm {printable} --help does not document its own help flag:\n{stdout}"
        );
        assert!(
            !stdout.contains("see more with '--help'"),
            "csm {printable} --help printed the SHORT help screen (that hint only \
             appears in `-h` output):\n{stdout}"
        );

        let mut missing: Vec<String> = required_help_tokens(cmd)
            .into_iter()
            .filter(|token| !stdout.contains(token))
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "csm {printable} --help does not render every argument the definition \
             declares: {missing:?}\nFull help output:\n{stdout}"
        );
    }
}
