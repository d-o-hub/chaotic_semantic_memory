// Unit tests for change discovery. Included by `lib.rs` as `mod changed_tests`.
//
// Git itself is exercised by the acceptance tests in `tests/`; here the runner
// is scripted so the failure directions (unresolvable ref, no merge base,
// unreadable diff, non-UTF-8 output, malformed records) are provable without a
// repository — and so the exact `git` argv contract is pinned.

use std::collections::HashMap;


/// A `GitRunner` that answers from a script and records every command.
struct ScriptedGit {
    answers: HashMap<String, Result<Vec<u8>, String>>,
    seen: std::cell::RefCell<Vec<String>>,
}

impl ScriptedGit {
    fn new(answers: &[(&str, Result<&str, &str>)]) -> Self {
        Self {
            answers: answers
                .iter()
                .map(|(command, answer)| {
                    let answer = match answer {
                        Ok(stdout) => Ok(stdout.as_bytes().to_vec()),
                        Err(stderr) => Err((*stderr).to_owned()),
                    };
                    ((*command).to_owned(), answer)
                })
                .collect(),
            seen: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn commands(&self) -> Vec<String> {
        self.seen.borrow().clone()
    }
}

impl GitRunner for ScriptedGit {
    fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let command = args.join(" ");
        self.seen.borrow_mut().push(command.clone());
        match self.answers.get(&command) {
            Some(Ok(stdout)) => Ok(stdout.clone()),
            Some(Err(stderr)) => Err(stderr.clone()),
            None => Err(format!("unscripted command: {command}")),
        }
    }
}

fn name_status(records: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend_from_slice(record.as_bytes());
        bytes.push(0);
    }
    bytes
}

#[test]
fn parses_added_modified_and_deleted() {
    let files = parse_name_status(&name_status(&["A\0src/new.rs", "M\0src/lib.rs", "D\0src/gone.rs"]))
        .expect("well-formed name-status parses");
    assert_eq!(
        files,
        vec![
            ChangedFile {
                path: "src/new.rs".to_owned(),
                kind: ChangeKind::Added
            },
            ChangedFile {
                path: "src/lib.rs".to_owned(),
                kind: ChangeKind::Modified
            },
            ChangedFile {
                path: "src/gone.rs".to_owned(),
                kind: ChangeKind::Deleted
            },
        ]
    );
}

#[test]
fn a_type_change_and_an_unknown_status_are_content_changes() {
    // 'T' (file type changed) and any future status must keep the path, because
    // dropping it would turn an unknown change into a skip.
    let files = parse_name_status(&name_status(&["T\0src/lib.rs", "X\0src/odd.rs"]))
        .expect("parses");
    assert!(
        files
            .iter()
            .all(|file| file.kind == ChangeKind::Modified),
        "unexpected kinds: {files:?}"
    );
}

#[test]
fn a_rename_records_both_sides() {
    let files = parse_name_status(&name_status(&["R100\0src/old.rs\0src/new.rs"]))
        .expect("parses");
    assert_eq!(
        files,
        vec![
            ChangedFile {
                path: "src/old.rs".to_owned(),
                kind: ChangeKind::Deleted
            },
            ChangedFile {
                path: "src/new.rs".to_owned(),
                kind: ChangeKind::Renamed {
                    from: "src/old.rs".to_owned()
                }
            },
        ],
        "a rule matching either side of the move must trigger"
    );
}

#[test]
fn a_copy_records_the_destination() {
    let files = parse_name_status(&name_status(&["C75\0src/a.rs\0src/b.rs"])).expect("parses");
    assert_eq!(files.len(), 2);
    assert_eq!(files[1].path, "src/b.rs");
    assert_eq!(
        files[1].kind,
        ChangeKind::Renamed {
            from: "src/a.rs".to_owned()
        }
    );
}

#[test]
fn an_empty_diff_is_an_empty_change_set() {
    assert!(parse_name_status(&[]).expect("parses").is_empty());
}

#[test]
fn malformed_records_are_errors_not_empty_sets() {
    let missing_path = parse_name_status(&name_status(&["M"])).expect_err("status without a path");
    assert!(
        missing_path.contains("without a path"),
        "diagnostic should name the shape, got: {missing_path}"
    );

    let missing_move_sides =
        parse_name_status(&name_status(&["R100\0src/old.rs"])).expect_err("rename without two paths");
    assert!(
        missing_move_sides.contains("source and destination"),
        "diagnostic should name the shape, got: {missing_move_sides}"
    );
}

#[test]
fn non_utf8_paths_are_errors() {
    let bytes = b"M\0src/\xff\xfe.rs\0".to_vec();
    let error = parse_name_status(&bytes).expect_err("non-UTF-8 path cannot be classified");
    assert!(error.contains("non-UTF-8"), "got: {error}");
}

#[test]
fn from_paths_normalizes_sorts_and_dedupes() {
    let changed = ChangedFiles::from_paths([
        "./src/b.rs",
        "src/a.rs",
        "src/b.rs",
        "",
        "src\\c.rs",
    ]);
    assert_eq!(changed.paths(), vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
    assert_eq!(changed.len(), 3);
    assert!(!changed.is_empty());
    assert!(!changed.is_failed());
    assert_eq!(changed.error(), None);
    // The record accessor is what the policy walks: an empty leak here would
    // classify every path as unclassified while `paths()` still looked right.
    let files = changed.files();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0].path, "src/a.rs");
    assert_eq!(files[0].kind, ChangeKind::Modified);
    assert_eq!(files[2].path, "src/c.rs");
}

#[test]
fn diagnostics_name_the_record_that_offended() {
    // The record number is the only positional clue a failing CI run gives a
    // reader, so it is asserted rather than ignored.
    let non_utf8_status = b"M\0src/a.rs\0\xff\xfe\0".to_vec();
    let error = parse_name_status(&non_utf8_status).expect_err("non-UTF-8 status");
    assert!(error.contains("at record 2"), "got: {error}");

    let non_utf8_path = b"A\0\xff\xfe\0".to_vec();
    let error = parse_name_status(&non_utf8_path).expect_err("non-UTF-8 path");
    assert!(error.contains("at record 1"), "got: {error}");

    let non_utf8_move_destination = b"R100\0src/old.rs\0\xff\xfe\0".to_vec();
    let error = parse_name_status(&non_utf8_move_destination).expect_err("non-UTF-8 destination");
    assert!(error.contains("at record 2"), "got: {error}");

    // A move consumes two path tokens, so a fault after it is numbered from
    // there: an index that stops advancing would blame an earlier record.
    let non_utf8_status_after_a_move = b"R100\0src/old.rs\0src/new.rs\0\xff\xfe\0".to_vec();
    let error =
        parse_name_status(&non_utf8_status_after_a_move).expect_err("non-UTF-8 status after a move");
    assert!(error.contains("at record 3"), "got: {error}");
}

#[test]
fn process_git_returns_stdout_and_fails_closed() {
    // The scripted runner above pins the argv contract; this exercises the real
    // `ProcessGit` in a throwaway repository, so the exit-status check and the
    // stdout it returns are proven rather than assumed.
    let repo = tempfile::tempdir().expect("temp dir");
    let git = ProcessGit::new(repo.path());

    git.run(&["init", "--quiet", "--initial-branch=main"])
        .expect("init a repository");

    let stdout = git
        .run(&["rev-parse", "--is-inside-work-tree"])
        .expect("runs inside the new repository");
    let stdout = String::from_utf8(stdout).expect("git prints UTF-8 here");
    assert_eq!(stdout.trim(), "true");

    let missing = git
        .run(&["rev-parse", "--verify", "refs/heads/no-such-branch"])
        .expect_err("a failing git command must not read as success");
    assert!(missing.contains("failed"), "got: {missing}");
    assert!(
        missing.contains("rev-parse"),
        "the diagnostic should name the command, got: {missing}"
    );
}

#[test]
fn failed_discovery_carries_its_diagnostic() {
    let changed = ChangedFiles::failed("no merge base");
    assert!(changed.is_failed());
    assert!(changed.is_empty());
    assert_eq!(changed.error(), Some("no merge base"));
}

#[test]
fn discover_since_resolves_the_merge_base_range() {
    let git = ScriptedGit::new(&[
        ("rev-parse --verify --quiet --end-of-options origin/main^{commit}", Ok("1111111111111111111111111111111111111111\n")),
        ("rev-parse --verify --quiet --end-of-options HEAD^{commit}", Ok("2222222222222222222222222222222222222222\n")),
        ("merge-base 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222", Ok("3333333333333333333333333333333333333333\n")),
        ("diff --name-status -z 3333333333333333333333333333333333333333 2222222222222222222222222222222222222222", Ok("M\0src/cli/mcp.rs\0")),
    ]);
    let discovery = discover_since(&git, "origin/main");
    let range = discovery.range.expect("range resolved");
    assert_eq!(range.requested, "origin/main");
    assert_eq!(range.merge_base_sha, "3333333333333333333333333333333333333333");
    assert_eq!(discovery.changed.paths(), vec!["src/cli/mcp.rs"]);
    assert_eq!(
        git.commands(),
        vec![
            "rev-parse --verify --quiet --end-of-options origin/main^{commit}",
            "rev-parse --verify --quiet --end-of-options HEAD^{commit}",
            "merge-base 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222",
            "diff --name-status -z 3333333333333333333333333333333333333333 2222222222222222222222222222222222222222",
        ],
        "the diff must be taken against the merge base, not the base ref"
    );
}

#[test]
fn an_unresolvable_ref_is_a_failed_discovery_naming_the_ref() {
    let git = ScriptedGit::new(&[(
        "rev-parse --verify --quiet --end-of-options origin/typo^{commit}",
        Err("fatal: Needed a single revision"),
    )]);
    let discovery = discover_since(&git, "origin/typo");
    assert!(discovery.range.is_none());
    assert!(discovery.changed.is_failed());
    let error = discovery.changed.error().expect("diagnostic");
    assert!(error.contains("origin/typo"), "got: {error}");
    assert!(error.contains("Needed a single revision"), "got: {error}");
}

#[test]
fn an_unborn_head_is_a_failed_discovery() {
    let git = ScriptedGit::new(&[
        ("rev-parse --verify --quiet --end-of-options origin/main^{commit}", Ok("1111111111111111111111111111111111111111\n")),
        ("rev-parse --verify --quiet --end-of-options HEAD^{commit}", Err("fatal: Needed a single revision")),
    ]);
    let discovery = discover_since(&git, "origin/main");
    assert!(discovery.changed.is_failed());
    assert!(discovery.changed.error().expect("diagnostic").contains("HEAD"));
}

#[test]
fn unrelated_histories_fail_closed() {
    for answer in [
        Err("fatal: Not a valid object name"),
        Ok("\n"),
    ] {
        let git = ScriptedGit::new(&[
            ("rev-parse --verify --quiet --end-of-options origin/main^{commit}", Ok("1111111111111111111111111111111111111111\n")),
            ("rev-parse --verify --quiet --end-of-options HEAD^{commit}", Ok("2222222222222222222222222222222222222222\n")),
            ("merge-base 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222", answer),
        ]);
        let discovery = discover_since(&git, "origin/main");
        assert!(
            discovery.changed.is_failed(),
            "a merge base that cannot be computed must not look like an empty diff"
        );
        assert!(
            discovery
                .changed
                .error()
                .expect("diagnostic")
                .contains("no merge base")
        );
    }
}

#[test]
fn an_unreadable_diff_fails_closed() {
    let git = ScriptedGit::new(&[
        ("rev-parse --verify --quiet --end-of-options origin/main^{commit}", Ok("1111111111111111111111111111111111111111\n")),
        ("rev-parse --verify --quiet --end-of-options HEAD^{commit}", Ok("2222222222222222222222222222222222222222\n")),
        ("merge-base 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222", Ok("3333333333333333333333333333333333333333\n")),
        ("diff --name-status -z 3333333333333333333333333333333333333333 2222222222222222222222222222222222222222", Err("fatal: bad object 3333")),
    ]);
    let discovery = discover_since(&git, "origin/main");
    assert!(discovery.changed.is_failed());
    let error = discovery.changed.error().expect("diagnostic");
    assert!(error.contains("bad object"), "got: {error}");
}

#[test]
fn a_malformed_diff_fails_closed() {
    let git = ScriptedGit::new(&[
        ("rev-parse --verify --quiet --end-of-options origin/main^{commit}", Ok("1111111111111111111111111111111111111111\n")),
        ("rev-parse --verify --quiet --end-of-options HEAD^{commit}", Ok("2222222222222222222222222222222222222222\n")),
        ("merge-base 1111111111111111111111111111111111111111 2222222222222222222222222222222222222222", Ok("3333333333333333333333333333333333333333\n")),
        ("diff --name-status -z 3333333333333333333333333333333333333333 2222222222222222222222222222222222222222", Ok("M\0")),
    ]);
    let discovery = discover_since(&git, "origin/main");
    assert!(discovery.changed.is_failed());
}

#[test]
fn change_kinds_have_stable_labels() {
    assert_eq!(ChangeKind::Untracked.as_str(), "untracked");
    assert_eq!(
        ChangeKind::Renamed {
            from: "a".to_owned()
        }
        .as_str(),
        "renamed"
    );
}
