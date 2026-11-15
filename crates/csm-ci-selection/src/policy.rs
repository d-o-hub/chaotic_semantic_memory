//! The declared policy: which change selects which CI job, and why.
//!
//! Everything here is data read by both the selector and its tests, so the
//! shipped decision cannot drift from the documented one. Patterns are globs
//! matched with [`MATCH_OPTIONS`]: case-sensitive, and `*`/`?` do **not** cross
//! `/` — `**` is the recursive form. `src/**/*.rs` therefore covers `src/lib.rs`
//! and `src/cli/commands/run.rs` alike, and `crates/*/src/**` stays inside one
//! crate directory.
//!
//! Evaluation order for a path: the job's own patterns first, then the shared
//! build inputs (`SHARED_BUILD_INPUTS`) when the job opts in, then
//! [`NON_BUILD_PATHS`]. A path that matches none of those is *unclassified*, and
//! an unclassified path selects every job — a new top-level directory must not
//! become a silent skip.
//!
//! The exclusions are per job and apply after inclusion: a path that matches an
//! include but also an exclude does not select that job, and does not veto other
//! paths that did match.

use glob::{MatchOptions, Pattern};

/// Glob semantics for every policy pattern.
///
/// `require_literal_separator` is the load-bearing flag: without it `*` crosses
/// directory boundaries and `src/*` would silently match `src/cli/run.rs`,
/// making the policy far broader than it reads.
pub const MATCH_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// A policy pattern that has been validated and compiled.
#[derive(Debug, Clone)]
pub struct PathPattern {
    raw: String,
    compiled: Pattern,
}

impl PathPattern {
    /// Validate and compile `raw`.
    ///
    /// Rejected: an empty pattern, an absolute pattern, a pattern containing a
    /// `..` component or a backslash, and anything `glob` cannot parse. A policy
    /// that cannot be compiled fails the process rather than classifying by a
    /// partially-applied rule.
    pub fn new(raw: &str) -> Result<Self, PolicyError> {
        if raw.is_empty() {
            return Err(PolicyError::new("empty path pattern"));
        }
        if raw.starts_with('/') {
            return Err(PolicyError::new(format!(
                "absolute path pattern: `{raw}` (patterns are repository-relative)"
            )));
        }
        if raw.contains('\\') {
            return Err(PolicyError::new(format!(
                "path pattern `{raw}` uses a backslash; use `/` separators"
            )));
        }
        if raw.split('/').any(|segment| segment == "..") {
            return Err(PolicyError::new(format!(
                "path pattern `{raw}` escapes the repository with `..`"
            )));
        }
        let compiled = Pattern::new(raw)
            .map_err(|err| PolicyError::new(format!("unparsable path pattern `{raw}`: {err}")))?;
        Ok(Self {
            raw: raw.to_owned(),
            compiled,
        })
    }

    /// The pattern as declared.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// True when `path` (repository-relative, `/`-separated) matches.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        self.compiled.matches_with(path, MATCH_OPTIONS)
    }
}

/// A policy/configuration failure. Always fatal: it means the selector cannot
/// state which jobs a change needs, which is never a reason to skip one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError {
    message: String,
}

impl PolicyError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PolicyError {}

/// How a job behaves on a push to `main` (the trunk).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrunkPolicy {
    /// Always runs on the trunk: the job protects an optional crate or an
    /// artifact whose breakage is only visible after merge.
    Always,
    /// Never runs on the trunk (pull-request-only evidence).
    Never,
    /// Runs on the trunk exactly when its path patterns match.
    ByPaths,
}

/// One CI job's selection rule, as declared.
#[derive(Debug, Clone, Copy)]
pub struct JobSpec<'a> {
    /// The `detect-changes` output key and the job family it gates.
    pub name: &'a str,
    /// Why this job exists and what its patterns cover. Printed by `--explain`.
    pub why: &'a str,
    /// Patterns that select this job.
    pub when_changed: &'a [&'a str],
    /// Patterns that unselect it again, applied after `when_changed`.
    pub excludes: &'a [&'a str],
    /// Also select this job for [`SHARED_BUILD_INPUTS`].
    pub shared_inputs: bool,
    /// Behaviour on a push to `main`.
    pub trunk: TrunkPolicy,
}

/// Inputs that can change what *any* build of this workspace produces.
///
/// Every specialized job compiles workspace crates — `test-duckdb` builds
/// `csm-duckdb` and its dependencies, `mcp-feature` builds the root crate,
/// `build-cli-cross` cross-builds the `csm` binary, `wasm` checks `csm-wasm`,
/// `fuzz-build` checks the fuzz workspace against the root crate. A path alone
/// does not prove which crate a change reaches (that needs a resolved dependency
/// graph, and the root crate is a dependency of most of them), so these select
/// every specialized job. This is the conservative direction issue #992 asked
/// for: over-running a job costs minutes, a skipped gate costs a regression.
pub const SHARED_BUILD_INPUTS: &[&str] = &[
    "**/*.rs",
    "**/Cargo.toml",
    "**/Cargo.lock",
    "**/build.rs",
    "rust-toolchain*",
    ".cargo/**",
    ".github/workflows/**",
    "flake.nix",
    "flake.lock",
];

/// Paths that cannot change any compiled artifact: documentation, plans,
/// skills, progress notes and repository tooling.
///
/// A change set made up only of these selects no specialized job. This is the
/// entire saving the selector buys today, and it is the safe kind: nothing here
/// is an input to `cargo build`, `cargo test`, the nix dev shell or a job's own
/// script. Individual job patterns are evaluated first, so a job whose gate is a
/// script still names that script (the wasm gate names three).
pub const NON_BUILD_PATHS: &[&str] = &[
    "**/*.md",
    "docs/**",
    "agents-docs/**",
    "plans/**",
    "progress/**",
    "book/**",
    ".agents/**",
    "LICENSE*",
    "README*",
    "SECURITY*",
    "CHANGELOG*",
    "llms.txt",
    "llms-full.txt",
    ".github/ISSUE_TEMPLATE/**",
    ".github/PULL_REQUEST_TEMPLATE*",
    ".github/dependabot.yml",
    "scripts/**",
    ".githooks/**",
    // Repository policy files: they gate the `lint` job (which runs
    // unconditionally), not a compiled artifact.
    "deny.toml",
    "rustfmt.toml",
    "clippy.toml",
    ".github/CODEOWNERS",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".qoder/**",
];

/// The seven outputs `detect-changes` publishes, in the order `ci.yml` declares
/// them. A job missing here is a job the workflow can never gate on names.
pub const JOBS: &[JobSpec<'static>] = &[
    JobSpec {
        name: "duckdb",
        why: "`test-duckdb` runs `cargo test -p csm-duckdb --all-features`, which \
              compiles csm-duckdb and the workspace crates it depends on",
        when_changed: &["crates/csm-duckdb/**"],
        excludes: &[],
        shared_inputs: true,
        trunk: TrunkPolicy::Always,
    },
    JobSpec {
        name: "wasm",
        why: "`wasm` checks csm-wasm for wasm32-unknown-unknown and runs the size gate \
              against the built package",
        when_changed: &[
            "crates/csm-wasm/**",
            "wasm/**",
            "scripts/build-wasm.sh",
            "scripts/check-wasm-freshness.sh",
            "scripts/wasm_size_gate.sh",
        ],
        excludes: &[],
        shared_inputs: true,
        trunk: TrunkPolicy::Always,
    },
    JobSpec {
        name: "fuzz",
        why: "`fuzz-build` runs `cargo check --manifest-path fuzz/Cargo.toml`, which \
              resolves the fuzz targets against the root crate",
        when_changed: &["fuzz/**"],
        excludes: &[],
        shared_inputs: true,
        trunk: TrunkPolicy::Always,
    },
    JobSpec {
        name: "mcp",
        why: "`mcp-feature` runs `cargo test -p chaotic_semantic_memory --features mcp`; \
              the MCP surface spans the server module, the CLI command and both \
              integration targets (issue #992: `src/cli/mcp.rs` and `tests/mcp*` were \
              not covered, so an MCP change skipped the job)",
        when_changed: &["src/mcp/**", "src/cli/mcp.rs", "tests/mcp*"],
        excludes: &[],
        shared_inputs: true,
        trunk: TrunkPolicy::Always,
    },
    JobSpec {
        name: "cli_cross",
        why: "`build-cli-cross` cross-builds the `csm` binary and `test-core-arm64` \
              runs its tests, so the CLI surface and the npm wrapper select it",
        when_changed: &[
            "src/bin/**",
            "src/cli/**",
            "crates/csm-cli/**",
            "cli-npm/**",
        ],
        excludes: &[],
        shared_inputs: true,
        trunk: TrunkPolicy::Always,
    },
    JobSpec {
        name: "mutation",
        why: "mutation evidence is required only where mutants can exist: production \
              Rust under `src/**` or `crates/*/src/**`, never tests, manifests or \
              tooling, and never on a push to main",
        when_changed: &[
            "src/**/*.rs",
            "crates/*/src/**/*.rs",
            "scripts/mutation_test.sh",
        ],
        excludes: &["**/tests/**", "**/tests.rs", "**/*_tests.rs"],
        shared_inputs: false,
        trunk: TrunkPolicy::Never,
    },
    JobSpec {
        name: "rust_src",
        why: "`miri` and the arm64 core tests follow any Rust source, manifest or \
              build-script change",
        when_changed: &["**/*.rs", "**/Cargo.toml", "**/Cargo.lock", "**/build.rs"],
        excludes: &[],
        shared_inputs: false,
        trunk: TrunkPolicy::ByPaths,
    },
];

/// What a path did to one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobMatch {
    /// The path selected the job, through this pattern.
    Matched {
        /// The declared pattern that matched.
        pattern: String,
    },
    /// The path matched an include but an exclusion took it back.
    Excluded {
        /// The declared exclusion that matched.
        pattern: String,
    },
    /// The path is irrelevant to this job.
    NoMatch,
}

/// A compiled job rule.
#[derive(Debug, Clone)]
pub struct Job {
    name: String,
    why: String,
    includes: Vec<PathPattern>,
    excludes: Vec<PathPattern>,
    trunk: TrunkPolicy,
}

impl Job {
    /// The `detect-changes` output key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Why this job exists, for reports.
    #[must_use]
    pub fn why(&self) -> &str {
        &self.why
    }

    /// Behaviour on a push to `main`.
    #[must_use]
    pub const fn trunk(&self) -> TrunkPolicy {
        self.trunk
    }

    /// Classify one path against this job's rule.
    #[must_use]
    pub fn classify(&self, path: &str) -> JobMatch {
        let Some(included) = self.includes.iter().find(|pattern| pattern.matches(path)) else {
            return JobMatch::NoMatch;
        };
        if let Some(excluded) = self.excludes.iter().find(|pattern| pattern.matches(path)) {
            return JobMatch::Excluded {
                pattern: excluded.as_str().to_owned(),
            };
        }
        JobMatch::Matched {
            pattern: included.as_str().to_owned(),
        }
    }
}

/// The compiled policy: job rules plus the non-build path set.
#[derive(Debug, Clone)]
pub struct Policy {
    jobs: Vec<Job>,
    non_build: Vec<PathPattern>,
}

impl Policy {
    /// Compile the shipped policy ([`JOBS`], [`NON_BUILD_PATHS`],
    /// [`SHARED_BUILD_INPUTS`]).
    pub fn compile() -> Result<Self, PolicyError> {
        Self::from_tables(JOBS, NON_BUILD_PATHS, SHARED_BUILD_INPUTS)
    }

    /// Compile a policy from explicit tables (tests and fixtures use this to
    /// exercise the compiler's failure directions).
    pub fn from_tables(
        jobs: &[JobSpec<'_>],
        non_build: &[&str],
        shared_inputs: &[&str],
    ) -> Result<Self, PolicyError> {
        if jobs.is_empty() {
            return Err(PolicyError::new(
                "policy declares no jobs; an empty policy would skip every gate",
            ));
        }
        let mut compiled_jobs = Vec::with_capacity(jobs.len());
        for spec in jobs {
            if spec.name.is_empty() {
                return Err(PolicyError::new("policy job with an empty name"));
            }
            if compiled_jobs.iter().any(|job: &Job| job.name == spec.name) {
                return Err(PolicyError::new(format!(
                    "duplicate policy job `{}`: two rules writing one output key",
                    spec.name
                )));
            }
            let shared = if spec.shared_inputs {
                compile_all(shared_inputs, "shared build inputs")?
            } else {
                Vec::new()
            };
            let mut includes = compile_all(spec.when_changed, spec.name)?;
            includes.extend(shared);
            if includes.is_empty() {
                return Err(PolicyError::new(format!(
                    "policy job `{}` has no patterns; a job nothing selects is a job that \
                     can never run",
                    spec.name
                )));
            }
            compiled_jobs.push(Job {
                name: spec.name.to_owned(),
                why: spec.why.to_owned(),
                includes,
                excludes: compile_all(spec.excludes, spec.name)?,
                trunk: spec.trunk,
            });
        }
        Ok(Self {
            jobs: compiled_jobs,
            non_build: compile_all(non_build, "non-build paths")?,
        })
    }

    /// The compiled job rules, in declaration order.
    #[must_use]
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// True when the path cannot affect a compiled artifact.
    #[must_use]
    pub fn is_non_build(&self, path: &str) -> bool {
        self.non_build.iter().any(|pattern| pattern.matches(path))
    }
}

fn compile_all(patterns: &[&str], owner: &str) -> Result<Vec<PathPattern>, PolicyError> {
    patterns
        .iter()
        .map(|raw| {
            PathPattern::new(raw)
                .map_err(|err| PolicyError::new(format!("{} (in {owner})", err.message())))
        })
        .collect()
}
