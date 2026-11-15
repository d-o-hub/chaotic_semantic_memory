// Unit tests for the selection mapping. Included by `lib.rs` as `mod selection_tests`.


fn policy() -> Policy {
    Policy::compile().expect("the shipped policy compiles")
}

fn selection_for(paths: &[&str], mode: Mode) -> Selection {
    let changed = ChangedFiles::from_paths(paths);
    select(&policy(), &changed, mode)
}

#[test]
fn a_documentation_only_change_selects_nothing() {
    let selection = selection_for(&["docs/guide.md", "plans/ACTIONS.md"], Mode::PullRequest);
    assert!(
        selection.selected().is_empty(),
        "docs cannot change a compiled artifact: {:?}",
        selection.selected()
    );
    assert_eq!(selection.skipped().len(), JOBS.len());
    assert!(selection.notes().is_empty());
    assert_eq!(
        selection.reason("fuzz"),
        Some(NO_MATCH_REASON),
        "every skip states why"
    );
}

#[test]
fn a_failed_discovery_selects_every_job_with_the_diagnostic() {
    let changed = ChangedFiles::failed("`git merge-base` failed: fatal: Not a valid object name");
    let selection = select(&policy(), &changed, Mode::PullRequest);
    assert_eq!(selection.selected().len(), JOBS.len());
    assert!(selection.skipped().is_empty());
    for job in JOBS {
        assert_eq!(
            selection.reason(job.name),
            Some(FAIL_CLOSED_REASON),
            "{} must state the fail-closed reason",
            job.name
        );
    }
    let note = selection.notes().join(" ");
    assert!(note.contains("Not a valid object name"), "got: {note}");
}

#[test]
fn an_empty_change_set_selects_every_job() {
    // A successful empty diff is still no information about which jobs the run
    // needs, so it must not be read as "nothing changed".
    let selection = selection_for(&[], Mode::PullRequest);
    assert_eq!(selection.selected().len(), JOBS.len());
    assert_eq!(selection.reason("mcp"), Some(NO_CHANGE_INFORMATION_REASON));
}

#[test]
fn an_unclassified_path_selects_every_job_and_is_named() {
    let selection = selection_for(&["new-top-level/file.bin"], Mode::PullRequest);
    assert_eq!(selection.selected().len(), JOBS.len());
    assert_eq!(selection.reason("duckdb"), Some(UNCLASSIFIED_REASON));
    let note = selection.notes().join(" ");
    assert!(
        note.contains("new-top-level/file.bin"),
        "the note must name the path that forced full selection, got: {note}"
    );
}

#[test]
fn a_production_change_selects_every_job_and_a_test_change_skips_mutation() {
    let production = selection_for(&["src/cli/mcp.rs"], Mode::PullRequest);
    assert_eq!(production.selected().len(), JOBS.len());
    assert!(
        production
            .reason("mcp")
            .unwrap_or_default()
            .contains("src/cli/mcp.rs"),
        "the reason names the pattern that matched"
    );

    let tests_only = selection_for(&["tests/mcp_integration.rs"], Mode::PullRequest);
    assert!(tests_only.is_selected("mcp"));
    assert!(
        !tests_only.is_selected("mutation"),
        "an integration test cannot hold a mutant"
    );
    assert_eq!(
        tests_only.reason("mutation"),
        Some(NO_MATCH_REASON),
        "an integration target is outside the mutated trees entirely"
    );

    // An in-crate test module is a different shape: the mutation include
    // matched and an exclusion took it back, and the reason says so.
    let in_crate = selection_for(&["src/mcp/server_tests.rs"], Mode::PullRequest);
    assert!(!in_crate.is_selected("mutation"));
    assert_eq!(
        in_crate.reason("mutation"),
        Some("excluded by `**/*_tests.rs`"),
        "an excluded job states the exclusion"
    );
}

#[test]
fn trunk_mode_applies_the_push_policy() {
    let rust = selection_for(&["src/lib.rs"], Mode::Trunk);
    for name in ["duckdb", "wasm", "fuzz", "mcp", "cli_cross"] {
        assert_eq!(rust.reason(name), Some(TRUNK_ALWAYS_REASON), "{name}");
    }
    assert_eq!(
        rust.reason("mutation"),
        Some(TRUNK_NEVER_REASON),
        "mutation evidence is pull-request-only"
    );
    assert!(rust.is_selected("rust_src"));

    let docs = selection_for(&["docs/guide.md"], Mode::Trunk);
    for name in ["duckdb", "wasm", "fuzz", "mcp", "cli_cross"] {
        assert!(
            docs.is_selected(name),
            "{name} protects the trunk on every push"
        );
    }
    assert!(
        !docs.is_selected("rust_src"),
        "rust_src follows paths when the trunk push changed no Rust"
    );
    assert!(
        !docs.is_selected("mutation"),
        "docs never create mutation evidence"
    );
}

#[test]
fn selection_is_deterministic() {
    let paths = ["src/cli/mcp.rs", "docs/x.md", "tests/mcp_integration.rs"];
    let first = selection_for(&paths, Mode::PullRequest);
    let second = selection_for(&paths, Mode::PullRequest);
    assert_eq!(
        first, second,
        "same policy, same change set, same split with the same reasons"
    );
}

#[test]
fn outputs_cover_every_job_exactly_once_in_policy_order() {
    let selection = selection_for(&["src/lib.rs"], Mode::PullRequest);
    let outputs = selection.outputs(&policy());
    assert_eq!(outputs.len(), JOBS.len());
    assert_eq!(
        outputs.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
        JOBS.iter().map(|job| job.name).collect::<Vec<_>>()
    );
    assert!(
        outputs.iter().all(|(_, value)| *value),
        "a production Rust change selects every job"
    );

    let docs = selection_for(&["plans/ACTIONS.md"], Mode::PullRequest);
    assert!(
        docs.outputs(&policy()).iter().all(|(_, value)| !*value),
        "docs-only selects nothing, so every output is false"
    );
}

#[test]
fn select_all_is_the_single_definition_of_always_run() {
    let policy = policy();
    let selection = selection::select_all(
        &policy,
        ALL_JOBS_REASON,
        "--all: every job selected by request".to_owned(),
    );
    assert_eq!(selection.selected().len(), JOBS.len());
    assert!(selection.skipped().is_empty());
    assert_eq!(selection.reason("duckdb"), Some(ALL_JOBS_REASON));
    assert_eq!(selection.notes().len(), 1);
}
