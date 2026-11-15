// Unit tests for the declared policy. Included by `lib.rs` as `mod policy_tests`.
//
// The classification table below *is* the contract #992 is about: each row is a
// real change set from the repository, and the expected job set. If the policy
// table is edited, a row here fails first.


fn selected_for(paths: &[&str]) -> Vec<String> {
    let policy = Policy::compile().expect("the shipped policy compiles");
    let changed = ChangedFiles::from_paths(paths);
    select(&policy, &changed, Mode::PullRequest)
        .selected()
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

fn all_jobs() -> Vec<String> {
    JOBS.iter().map(|job| job.name.to_owned()).collect()
}

const ALL_JOB_NAMES: &[&str] = &[
    "duckdb",
    "wasm",
    "fuzz",
    "mcp",
    "cli_cross",
    "mutation",
    "rust_src",
];

fn everything_but(missing: &[&str]) -> Vec<&'static str> {
    ALL_JOB_NAMES
        .iter()
        .copied()
        .filter(|name| !missing.contains(name))
        .collect()
}

#[test]
fn the_shipped_policy_compiles_with_the_seven_outputs_the_workflow_declares() {
    let policy = Policy::compile().expect("compiles");
    let names = policy
        .jobs()
        .iter()
        .map(|job| job.name().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "duckdb",
            "wasm",
            "fuzz",
            "mcp",
            "cli_cross",
            "mutation",
            "rust_src"
        ],
        "these are the `detect-changes` output keys ci.yml publishes"
    );
}

#[test]
fn every_job_explains_itself_in_prose() {
    // `why()` is the reviewer-facing justification in the selection report, so a
    // placeholder value (`""`, `"xyzzy"`) is what a mutant of that accessor
    // produces — and a reason shorter than a clause is not a reason.
    let policy = Policy::compile().expect("compiles");
    for job in policy.jobs() {
        let why = job.why();
        assert!(
            why.len() >= 30,
            "`{}` carries no usable reason: {why}",
            job.name()
        );
    }
}

#[test]
fn the_mcp_cli_and_integration_tests_select_the_mcp_job() {
    // Issue #992: `src/cli/mcp.rs` and both `tests/mcp*_integration.rs` files did
    // not match the old `^src/mcp/` rule, so the dedicated MCP job was skipped.
    for path in [
        "src/cli/mcp.rs",
        "tests/mcp_integration.rs",
        "tests/mcp_streamable_http_integration.rs",
        "src/mcp/server.rs",
        "src/mcp/tools.rs",
    ] {
        assert!(
            selected_for(&[path]).contains(&"mcp".to_owned()),
            "{path} must select the mcp job"
        );
    }
}

#[test]
fn the_classification_table_matches_the_shipped_policy() {
    let cases: &[(&str, Vec<&str>)] = &[
        // A production Rust module: every specialized build job, plus mutation
        // evidence and the Rust-source jobs.
        ("src/cli/mcp.rs", all_of()),
        ("src/mcp/server_tests.rs", everything_but(&["mutation"])),
        ("tests/mcp_integration.rs", everything_but(&["mutation"])),
        ("crates/csm-memory/src/singularity_search.rs", all_of()),
        ("crates/csm-duckdb/src/lib.rs", all_of()),
        ("src/bin/csm.rs", all_of()),
        (
            "fuzz/fuzz_targets/target.rs",
            everything_but(&["mutation"]),
        ),
        // A manifest is a build input but cannot hold a mutant.
        (
            "crates/csm-memory/Cargo.toml",
            everything_but(&["mutation"]),
        ),
        ("Cargo.lock", everything_but(&["mutation"])),
        // The workflow defines the jobs: it selects the build jobs, not the
        // Rust-source jobs.
        (
            ".github/workflows/ci.yml",
            vec!["duckdb", "wasm", "fuzz", "mcp", "cli_cross"],
        ),
        // A job's own gate script selects that job and nothing else.
        ("scripts/build-wasm.sh", vec!["wasm"]),
        ("scripts/mutation_test.sh", vec!["mutation"]),
        // Documentation, plans, skills and tooling select nothing.
        ("docs/release-guardrails.md", vec![]),
        ("agents-docs/coding-standards.md", vec![]),
        ("plans/GOAP_STATE.md", vec![]),
        ("progress/PROGRESS.md", vec![]),
        ("CHANGELOG.md", vec![]),
        ("llms.txt", vec![]),
        (".agents/skills/rust-development/SKILL.md", vec![]),
        ("scripts/validate.sh", vec![]),
        ("deny.toml", vec![]),
        (".gitignore", vec![]),
    ];
    for (path, expected) in cases {
        let selected = selected_for(&[path]);
        assert_eq!(
            selected,
            expected.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>(),
            "classification of {path}"
        );
    }
}

fn all_of() -> Vec<&'static str> {
    vec![
        "duckdb",
        "wasm",
        "fuzz",
        "mcp",
        "cli_cross",
        "mutation",
        "rust_src",
    ]
}

#[test]
fn mutation_never_covers_tests_manifests_or_tooling() {
    let policy = Policy::compile().expect("compiles");
    let mutation = policy
        .jobs()
        .iter()
        .find(|job| job.name() == "mutation")
        .expect("mutation job");
    // Tests are *excluded* (the include matched, an exclusion took it back);
    // everything else here matches no include at all. Neither selects the job.
    for path in [
        "src/framework_ops_tests.rs",
        "src/mcp/server_tests.rs",
        "crates/csm-retrieval/src/bm25/tests.rs",
        "crates/csm-retrieval/src/bm25/tests/cases.rs",
    ] {
        assert!(
            matches!(mutation.classify(path), JobMatch::Excluded { .. }),
            "{path} must be excluded from mutation evidence, got {:?}",
            mutation.classify(path)
        );
    }
    for path in [
        "Cargo.toml",
        "build.rs",
        "fuzz/fuzz_targets/target.rs",
        "tests/mcp_integration.rs",
    ] {
        assert_eq!(
            mutation.classify(path),
            JobMatch::NoMatch,
            "{path} cannot hold a mutant"
        );
    }
    for path in [
        "src/lib.rs",
        "src/cli/commands/run.rs",
        "crates/csm-retrieval/src/bm25.rs",
        "crates/csm-retrieval/src/bm25/index.rs",
    ] {
        assert!(
            matches!(mutation.classify(path), JobMatch::Matched { .. }),
            "{path} is production Rust and must create mutation evidence"
        );
    }
}

#[test]
fn a_non_rust_change_inside_a_crate_src_tree_does_not_create_mutation_evidence() {
    let policy = Policy::compile().expect("compiles");
    let mutation = policy
        .jobs()
        .iter()
        .find(|job| job.name() == "mutation")
        .expect("mutation job");
    assert_eq!(
        mutation.classify("crates/csm-memory/src/data/schema.json"),
        JobMatch::NoMatch
    );
}

#[test]
fn patterns_are_validated() {
    for bad in ["", "/etc/passwd", "src/../../etc", "src\\lib.rs"] {
        let error = PathPattern::new(bad).expect_err("must be rejected");
        assert!(
            !error.message().is_empty(),
            "rejection of `{bad}` needs a diagnostic"
        );
        // `Display` is what a failing workflow log shows; a `Default` formatter
        // would print an empty line where the reason should be.
        assert_eq!(
            error.to_string(),
            error.message(),
            "`{bad}` must display the diagnostic it carries"
        );
    }
    for good in ["src/**/*.rs", "**/*.md", "Cargo.lock", ".github/workflows/**"] {
        assert!(PathPattern::new(good).is_ok(), "`{good}` must be accepted");
    }
}

#[test]
fn a_star_does_not_cross_a_directory_boundary() {
    let pattern = PathPattern::new("src/*").expect("valid");
    assert!(pattern.matches("src/lib.rs"));
    assert!(
        !pattern.matches("src/cli/run.rs"),
        "`*` crossing `/` would make every policy pattern broader than it reads"
    );
    let crate_local = PathPattern::new("crates/*/src/**").expect("valid");
    assert!(crate_local.matches("crates/csm-memory/src/index.rs"));
    assert!(!crate_local.matches("crates/csm-memory/tests/index.rs"));
    assert!(!crate_local.matches("crates/nested/inner/src/index.rs"));
}

#[test]
fn a_recursive_pattern_covers_the_directory_root() {
    // `src/**/*.rs` must cover `src/lib.rs`, not only nested modules: the
    // mutation rule depends on it.
    let pattern = PathPattern::new("src/**/*.rs").expect("valid");
    assert!(pattern.matches("src/lib.rs"));
    assert!(pattern.matches("src/cli/commands/run.rs"));
}

#[test]
fn the_policy_compiler_rejects_tables_that_cannot_classify() {
    let base = JobSpec {
        name: "duckdb",
        why: "test job",
        when_changed: &["src/**"],
        excludes: &[],
        shared_inputs: false,
        trunk: TrunkPolicy::Always,
    };

    assert!(
        Policy::from_tables(&[], &["**/*.md"], &[]).is_err(),
        "an empty job table would skip every gate"
    );

    let unnamed = JobSpec {
        name: "",
        ..base
    };
    assert!(Policy::from_tables(&[unnamed], &[], &[]).is_err());

    let duplicated = [base, base];
    let error = Policy::from_tables(&duplicated, &[], &[])
        .expect_err("two rules writing one output key must be rejected");
    assert!(
        error.message().contains("duplicate"),
        "got: {}",
        error.message()
    );

    let patternless = JobSpec {
        name: "orphan",
        when_changed: &[],
        ..base
    };
    let error = Policy::from_tables(&[patternless], &[], &[])
        .expect_err("a job nothing selects can never run");
    assert!(
        error.message().contains("no patterns"),
        "got: {}",
        error.message()
    );

    let invalid = JobSpec {
        name: "duckdb",
        when_changed: &["/absolute/**"],
        ..base
    };
    let error =
        Policy::from_tables(&[invalid], &[], &[]).expect_err("invalid globs must not be silent");
    assert!(
        error.message().contains("absolute"),
        "got: {}",
        error.message()
    );
}

#[test]
fn job_names_are_usable_as_github_output_keys() {
    for job in JOBS {
        assert!(
            job.name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
            "`{}` is written as a `key=value` line into $GITHUB_OUTPUT",
            job.name
        );
        assert!(!job.why.is_empty(), "`{}` needs a stated reason", job.name);
    }
}

#[test]
fn non_build_paths_are_the_documented_set() {
    let policy = Policy::compile().expect("compiles");
    assert!(policy.is_non_build("README.md"));
    assert!(policy.is_non_build("scripts/anything.sh"));
    assert!(policy.is_non_build(".agents/skills/learn/SKILL.md"));
    assert!(!policy.is_non_build("src/lib.rs"));
    assert!(!policy.is_non_build("fuzz/fuzz_targets/x.rs"));
    assert!(
        !policy.is_non_build("new-top-level-directory/file.txt"),
        "an unknown path must stay unclassified so it selects every job"
    );
}
