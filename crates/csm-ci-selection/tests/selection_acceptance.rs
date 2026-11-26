//! Acceptance tests for the selector: real Git repositories, the real binary,
//! and the real workflow wiring.
//!
//! The unit tests prove the mapping; these prove the *hazard* #992 was opened
//! for — that a change touching the MCP CLI or an MCP integration target makes
//! the dedicated MCP job run — against a committed range in a temporary
//! repository, through `csm-ci-select` itself, in the shape `ci.yml` invokes it.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Output};

use csm_ci_selection::changed::{ChangeKind, ProcessGit, discover_since};
use csm_ci_selection::policy::Policy;
use csm_ci_selection::selection::{self, Mode};

/// Fixture plumbing for the acceptance cases below: throwaway repositories,
/// the built selector, and `$GITHUB_OUTPUT` parsing.
///
/// It lives in a `#[cfg(test)]` module because that is what `.clippy.toml`'s
/// `allow-expect-in-tests` recognises: at the root of an integration-test
/// crate a free function is not "in tests" for that lint, so a fixture that
/// cannot be created — no `git`, no temp dir, unreadable `ci.yml` — would be
/// a lint failure instead of the test failing, which is the intended outcome.
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

    /// A throwaway repository. Every Git call is env-isolated so the developer's (or
    /// the CI runner's) configuration cannot change what the fixture means.
    pub(super) struct Repo {
        dir: tempfile::TempDir,
    }

    impl Repo {
        pub(super) fn new() -> Self {
            let dir = tempfile::tempdir().expect("temp dir");
            let repo = Self { dir };
            repo.git(["init", "-q", "-b", "main"]);
            repo
        }

        pub(super) fn path(&self) -> &Path {
            self.dir.path()
        }

        pub(super) fn git<I, S>(&self, args: I) -> String
        where
            I: IntoIterator<Item = S>,
            S: AsRef<std::ffi::OsStr>,
        {
            let output = git_command(self.path())
                .args(args)
                .output()
                .expect("git runs");
            assert!(
                output.status.success(),
                "git failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).into_owned()
        }

        pub(super) fn write(&self, relative: &str, contents: &str) {
            let path = self.path().join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create parent");
            }
            std::fs::write(path, contents).expect("write fixture file");
        }

        pub(super) fn commit(&self, message: &str) {
            self.git(["add", "-A"]);
            self.git(["commit", "-q", "-m", message]);
        }

        /// The base commit: a Rust source and a doc, committed on `main`.
        pub(super) fn base() -> Self {
            let repo = Self::new();
            repo.write("src/lib.rs", "pub fn base() {}\n");
            repo.write("README.md", "# fixture\n");
            repo.commit("base");
            repo
        }

        /// Switch to a branch that changes the given paths.
        pub(super) fn branch_with(&self, name: &str, paths: &[(&str, &str)]) {
            self.git(["checkout", "-q", "-b", name]);
            for (path, contents) in paths {
                self.write(path, contents);
            }
            self.commit(name);
        }
    }

    pub(super) fn git_command(cwd: &Path) -> Command {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(cwd)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
            .env("GIT_TERMINAL_PROMPT", "0");
        command
    }

    pub(super) fn selector(cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_csm-ci-select"));
        command.args(args).current_dir(cwd);
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().expect("selector runs")
    }

    /// Parse `key=value` lines from stdout.
    pub(super) fn outputs(output: &Output) -> BTreeMap<String, bool> {
        assert!(
            output.status.success(),
            "selector failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (key, value) = line.split_once('=').expect("key=value line");
                (key.to_owned(), value == "true")
            })
            .collect()
    }

    /// Run the selector with `--github-output`, through a real `$GITHUB_OUTPUT`.
    pub(super) fn github_outputs(cwd: &Path, args: &[&str]) -> BTreeMap<String, bool> {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("github_output");
        std::fs::write(&file, "").expect("create output file");
        let file_string = file.to_string_lossy().into_owned();
        let output = selector(cwd, args, &[("GITHUB_OUTPUT", file_string.as_str())]);
        assert!(
            output.status.success(),
            "selector failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let written = std::fs::read_to_string(&file).expect("read output file");
        let parsed = written
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (key, value) = line.split_once('=').expect("key=value line");
                (key.to_owned(), value == "true")
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            parsed.len(),
            job_names().len(),
            "every job output must be written: {written}"
        );
        parsed
    }

    pub(super) fn all_false() -> BTreeMap<String, bool> {
        job_names()
            .into_iter()
            .map(|name| (name.to_owned(), false))
            .collect()
    }

    pub(super) fn all_true() -> BTreeMap<String, bool> {
        job_names()
            .into_iter()
            .map(|name| (name.to_owned(), true))
            .collect()
    }

    pub(super) fn with(names: &[&str]) -> BTreeMap<String, bool> {
        let mut map = all_false();
        for name in names {
            map.insert((*name).to_owned(), true);
        }
        map
    }
}

use harness::*;

#[test]
fn an_mcp_cli_or_integration_change_selects_the_mcp_job() {
    // The regression from #992: `src/cli/mcp.rs` and both MCP integration
    // targets missed the old `^src/mcp/` rule, so the dedicated `cargo test
    // --features mcp` job never ran for them. Acceptance 1 of #992 also wants
    // the CLI checks for the CLI's MCP command, and acceptance 3 the root
    // manifest, which reaches every shared-input job.
    //
    // The exact map is asserted, not just `mcp`: any Rust or manifest change
    // selects the jobs that compile workspace crates, the path families decide
    // `mcp`, and only production Rust decides `mutation`.
    let shared = ["duckdb", "wasm", "fuzz", "mcp", "cli_cross", "rust_src"];
    for (path, mutation) in [
        ("src/cli/mcp.rs", true),
        ("src/mcp/server.rs", true),
        ("tests/mcp_integration.rs", false),
        ("tests/mcp_streamable_http_integration.rs", false),
        ("Cargo.toml", false),
    ] {
        let repo = Repo::base();
        repo.branch_with("mcp-change", &[(path, "// mcp change\n")]);
        let outputs = github_outputs(repo.path(), &["--since", "main", "--github-output"]);
        let mut expected = with(&shared);
        expected.insert("mutation".to_owned(), mutation);
        assert_eq!(outputs, expected, "{path} expectation");
    }
}

#[test]
fn a_documentation_only_change_emits_all_outputs_false() {
    let repo = Repo::base();
    repo.branch_with(
        "docs",
        &[
            ("docs/guide.md", "# guide\n"),
            ("plans/ACTIONS.md", "# plan\n"),
        ],
    );
    let outputs = github_outputs(repo.path(), &["--since", "main", "--github-output"]);
    assert_eq!(outputs, all_false());
}

#[test]
fn an_unknown_path_selects_every_job() {
    let repo = Repo::base();
    repo.branch_with("new-area", &[("new-area/notes.txt", "unclassified\n")]);
    let output = selector(
        repo.path(),
        &["--since", "main", "--github-output"],
        &[("GITHUB_OUTPUT", "/dev/null")],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("new-area/notes.txt"),
        "the summary must name the unclassified path: {stderr}"
    );
    let outputs = github_outputs(repo.path(), &["--since", "main", "--github-output"]);
    assert_eq!(outputs, all_true());
}

#[test]
fn an_unresolvable_ref_runs_every_job_and_is_not_an_error() {
    let repo = Repo::base();
    repo.branch_with("rust", &[("src/lib.rs", "pub fn base() { let _ = 1; }\n")]);
    let outputs = github_outputs(
        repo.path(),
        &["--since", "origin/does-not-exist", "--github-output"],
    );
    assert_eq!(
        outputs,
        all_true(),
        "an unreadable range is not evidence that nothing changed"
    );
}

#[test]
fn a_poisoned_git_environment_cannot_change_the_classification() {
    let repo = Repo::base();
    repo.branch_with("docs", &[("docs/guide.md", "# guide\n")]);
    let output = selector(
        repo.path(),
        &["--since", "main", "--github-output"],
        &[
            ("GITHUB_OUTPUT", "/dev/null"),
            ("GIT_DIR", "/nonexistent/git-dir"),
            ("GIT_WORK_TREE", "/nonexistent/work-tree"),
        ],
    );
    assert!(output.status.success());
    let outputs = github_outputs(repo.path(), &["--since", "main", "--github-output"]);
    assert_eq!(
        outputs,
        all_false(),
        "an inherited GIT_DIR must not turn a docs-only diff into a failed discovery"
    );
}

#[test]
fn a_push_applies_the_trunk_policy() {
    let repo = Repo::base();
    let before = repo.git(["rev-parse", "HEAD"]).trim().to_owned();
    repo.branch_with("rust", &[("src/lib.rs", "pub fn base() { let _ = 2; }\n")]);
    let outputs = github_outputs(
        repo.path(),
        &["--since", &before, "--push", "--github-output"],
    );
    assert_eq!(
        outputs,
        with(&["duckdb", "wasm", "fuzz", "mcp", "cli_cross", "rust_src"])
    );
    assert!(
        !outputs["mutation"],
        "mutation evidence is pull-request-only"
    );

    // Trunk coverage is unconditional only for the optional crates: `rust_src`
    // stays path-driven and `mutation` stays event-driven, exactly as the
    // pre-#992 workflow behaved, so a documentation-only push must not claim
    // Rust-source coverage it did not exercise.
    let repo = Repo::base();
    let before = repo.git(["rev-parse", "HEAD"]).trim().to_owned();
    repo.branch_with("docs", &[("docs/guide.md", "# guide\n")]);
    let outputs = github_outputs(
        repo.path(),
        &["--since", &before, "--push", "--github-output"],
    );
    assert_eq!(
        outputs,
        with(&["duckdb", "wasm", "fuzz", "mcp", "cli_cross"]),
        "a documentation-only push protects the trunk without claiming Rust coverage"
    );
}

#[test]
fn the_path_list_input_classifies_a_pre_resolved_diff() {
    let repo = Repo::base();
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("paths0");
    let mut bytes = Vec::new();
    for path in ["src/cli/mcp.rs", "docs/guide.md"] {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
    }
    std::fs::write(&file, bytes).expect("write path list");
    let outputs = github_outputs(
        repo.path(),
        &["--paths0", &file.to_string_lossy(), "--github-output"],
    );
    assert!(outputs["mcp"]);
    assert!(outputs["mutation"], "src/cli/mcp.rs is production Rust");
}

#[test]
fn all_selects_every_job_without_a_change_set() {
    let repo = Repo::base();
    let outputs = github_outputs(repo.path(), &["--all", "--github-output"]);
    assert_eq!(outputs, all_true());
}

#[test]
fn usage_errors_exit_two_so_the_caller_runs_everything() {
    let repo = Repo::base();

    let no_input = selector(repo.path(), &[], &[]);
    assert_eq!(no_input.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&no_input.stderr).contains("--since"));

    let conflicting_outputs = selector(repo.path(), &["--all", "--json", "--github-output"], &[]);
    assert_eq!(conflicting_outputs.status.code(), Some(2));

    let missing_file = selector(repo.path(), &["--paths0", "/nonexistent/paths0"], &[]);
    assert_eq!(missing_file.status.code(), Some(2));

    let github_output_without_env = selector(
        repo.path(),
        &["--all", "--github-output"],
        &[("GITHUB_OUTPUT", "")],
    );
    assert_eq!(
        github_output_without_env.status.code(),
        Some(2),
        "writing GitHub outputs needs the file GitHub provides"
    );

    let unknown_flag = selector(repo.path(), &["--nope"], &[]);
    assert_eq!(unknown_flag.status.code(), Some(2));
}

#[test]
fn the_library_and_the_binary_agree_on_a_real_range() {
    let repo = Repo::base();
    repo.branch_with("rust", &[("src/cli/mcp.rs", "pub fn command() {}\n")]);

    let discovery = discover_since(&ProcessGit::new(repo.path()), "main");
    let range = discovery.range.clone().expect("range resolved");
    assert_eq!(range.requested, "main");
    assert!(
        !range.merge_base_sha.is_empty() && !range.head_sha.is_empty(),
        "the range records the commits it diffed"
    );
    let policy = Policy::compile().expect("policy compiles");
    let selection = selection::select(&policy, &discovery.changed, Mode::PullRequest);
    assert!(selection.is_selected("mcp"));

    let outputs = github_outputs(repo.path(), &["--since", "main", "--github-output"]);
    for job in policy.jobs() {
        assert_eq!(
            outputs[job.name()],
            selection.is_selected(job.name()),
            "the binary and the library must classify {} the same way",
            job.name()
        );
    }
}

#[test]
fn a_rename_classifies_the_destination_and_the_source() {
    let repo = Repo::base();
    repo.write("src/helper.rs", "pub fn helper() {}\n");
    repo.commit("add helper");
    std::fs::create_dir_all(repo.path().join("src/cli")).expect("destination directory");
    repo.git(["mv", "src/helper.rs", "src/cli/renamed.rs"]);
    repo.commit("move helper into the cli tree");

    let discovery = discover_since(&ProcessGit::new(repo.path()), "HEAD~1");
    let kinds = discovery
        .changed
        .files()
        .iter()
        .map(|file| (file.path.as_str(), file.kind.clone()))
        .collect::<Vec<_>>();
    assert!(
        kinds
            .iter()
            .any(|(path, kind)| *path == "src/cli/renamed.rs"
                && matches!(kind, ChangeKind::Renamed { .. })),
        "the destination must be classified: {kinds:?}"
    );
    assert!(
        kinds
            .iter()
            .any(|(path, kind)| *path == "src/helper.rs" && *kind == ChangeKind::Deleted),
        "the source of the move must be classified too: {kinds:?}"
    );
}

#[test]
fn the_classification_is_repeatable() {
    let repo = Repo::base();
    repo.branch_with("rust", &[("src/lib.rs", "pub fn base() { let _ = 3; }\n")]);
    let first = selector(repo.path(), &["--since", "main"], &[]);
    let second = selector(repo.path(), &["--since", "main"], &[]);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.status.code(), second.status.code());
}
