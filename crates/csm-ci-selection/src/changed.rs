//! The change set a CI run must classify, and how it is discovered.
//!
//! Two inputs are supported, mirroring how `ci.yml` obtains a diff:
//!
//! * a committed range — `discover_since` resolves the requested ref and `HEAD`
//!   to commits, takes their merge base, and reads
//!   `git diff --name-status -z <merge-base> <head>`, so only the paths the
//!   branch actually adds are classified (a plain two-dot diff against a moved
//!   base would report the base's own commits);
//! * a NUL-separated path list (`git diff --name-only -z`), for callers that
//!   already resolved the diff themselves.
//!
//! Failures are *data*, not empty sets: every Git step that cannot answer
//! produces [`ChangedFiles::failed`] with the command and its stderr, and
//! [`crate::selection::select`] turns that into "select every job". An empty
//! diff must never be inferred from an unreadable one.

use std::path::PathBuf;
use std::process::Command;

/// How a path changed, as reported by `git diff --name-status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    /// The path is the destination of a rename/copy; `from` is the source.
    Renamed {
        /// Source path of the rename/copy.
        from: String,
    },
    Untracked,
}

impl ChangeKind {
    /// Stable label for reports.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed { .. } => "renamed",
            Self::Untracked => "untracked",
        }
    }
}

/// One changed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repository-relative path, `/`-separated.
    pub path: String,
    /// How it changed.
    pub kind: ChangeKind,
}

/// The change set, or the reason it could not be discovered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangedFiles {
    files: Vec<ChangedFile>,
    discovery_failed: bool,
    discovery_error: Option<String>,
}

impl ChangedFiles {
    /// A discovered change set. Paths are normalized, deduplicated and sorted.
    #[must_use]
    pub fn new(files: Vec<ChangedFile>) -> Self {
        let mut files = files
            .into_iter()
            .map(|file| ChangedFile {
                path: normalize_path(&file.path),
                kind: file.kind,
            })
            .filter(|file| !file.path.is_empty())
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        files.dedup_by(|a, b| a.path == b.path && a.kind == b.kind);
        Self {
            files,
            discovery_failed: false,
            discovery_error: None,
        }
    }

    /// A change set from bare paths, each classified as [`ChangeKind::Modified`].
    #[must_use]
    pub fn from_paths<I, S>(paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::new(
            paths
                .into_iter()
                .map(|path| ChangedFile {
                    path: path.as_ref().to_owned(),
                    kind: ChangeKind::Modified,
                })
                .collect(),
        )
    }

    /// A change set that could not be discovered. The error is the diagnostic.
    #[must_use]
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            files: Vec::new(),
            discovery_failed: true,
            discovery_error: Some(error.into()),
        }
    }

    /// True when discovery failed and the set carries no usable paths.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        self.discovery_failed
    }

    /// The diagnostic of a failed discovery, when there was one.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.discovery_error.as_deref()
    }

    /// The changed files, sorted by path.
    #[must_use]
    pub fn files(&self) -> &[ChangedFile] {
        &self.files
    }

    /// The changed paths, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|file| file.path.as_str()).collect()
    }

    /// Number of changed files.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.files.len()
    }

    /// True when the discovered change set has no paths.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// `./`-prefixes and Windows separators normalized away, so policy globs need
/// one spelling.
fn normalize_path(path: &str) -> String {
    let path = path.trim();
    let path = path.replace('\\', "/");
    path.strip_prefix("./").map_or(path.clone(), str::to_owned)
}

/// The refs a range was resolved to, kept for reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRange {
    /// The ref the caller asked for, verbatim.
    pub requested: String,
    /// The commit that ref resolved to.
    pub base_sha: String,
    /// The commit `HEAD` resolved to.
    pub head_sha: String,
    /// Their merge base, which is what the diff is taken against.
    pub merge_base_sha: String,
}

/// A discovery result: the change set, plus the resolved range that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Present when the change set came from a committed range.
    pub range: Option<ResolvedRange>,
    /// The change set, or the reason it is unknown.
    pub changed: ChangedFiles,
}

/// Runs `git` and returns its stdout, or a diagnostic on any failure.
///
/// Abstracted so that the failure paths (unresolvable ref, no merge base,
/// unreadable repository, non-UTF-8 path) are unit-testable without a
/// repository, and so integration tests can drive real Git.
pub trait GitRunner {
    /// Run `git <args>` in the repository this runner is bound to.
    fn run(&self, args: &[&str]) -> Result<Vec<u8>, String>;
}

/// A [`GitRunner`] that shells out to the host `git`.
#[derive(Debug, Clone)]
pub struct ProcessGit {
    root: PathBuf,
}

impl ProcessGit {
    /// A runner bound to `root` (`git -C root ...`).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl GitRunner for ProcessGit {
    fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            // A CI step or an outer test may export these; inheriting them would
            // silently classify a different repository (or none).
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .map_err(|err| format!("cannot run `git {}`: {err}", args.join(" ")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            let detail = if detail.is_empty() {
                format!("exit status {}", output.status)
            } else {
                detail.to_owned()
            };
            return Err(format!("`git {}` failed: {detail}", args.join(" ")));
        }
        Ok(output.stdout)
    }
}

fn trimmed(bytes: Vec<u8>, what: &str) -> Result<String, String> {
    let text = String::from_utf8(bytes).map_err(|err| format!("{what} is not UTF-8: {err}"))?;
    Ok(text.trim().to_owned())
}

fn rev_parse(git: &dyn GitRunner, reference: &str) -> Result<String, String> {
    let spec = format!("{reference}^{{commit}}");
    let stdout = git
        .run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            spec.as_str(),
        ])
        .map_err(|err| format!("cannot resolve `{reference}` to a commit: {err}"))?;
    let sha = trimmed(stdout, &format!("`git rev-parse {spec}` output"))?;
    if sha.is_empty() {
        return Err(format!(
            "cannot resolve `{reference}` to a commit (empty `git rev-parse {spec}`)"
        ));
    }
    Ok(sha)
}

/// Resolve `<reference>...HEAD` and read the changed paths it introduces.
///
/// The diff is taken against the merge base, so a branch that is behind its
/// base classifies its own commits only. Every step that fails — the ref does
/// not resolve, `HEAD` has no commit, the histories are unrelated, the object
/// is missing from a shallow clone, Git cannot be spawned — returns a failed
/// [`ChangedFiles`] carrying that diagnostic.
#[must_use]
pub fn discover_since(git: &dyn GitRunner, reference: &str) -> Discovery {
    let failed = |error: String| Discovery {
        range: None,
        changed: ChangedFiles::failed(error),
    };

    let base_sha = match rev_parse(git, reference) {
        Ok(sha) => sha,
        Err(err) => return failed(err),
    };
    let head_sha = match rev_parse(git, "HEAD") {
        Ok(sha) => sha,
        Err(err) => return failed(err),
    };
    let merge_base_sha = match git.run(&["merge-base", base_sha.as_str(), head_sha.as_str()]) {
        Ok(bytes) => match trimmed(bytes, "`git merge-base` output") {
            Ok(sha) if !sha.is_empty() => sha,
            Ok(_) => {
                return failed(format!(
                    "no merge base between {reference} and HEAD (empty `git merge-base`)"
                ));
            }
            Err(err) => return failed(err),
        },
        Err(err) => {
            return failed(format!("no merge base between {reference} and HEAD: {err}"));
        }
    };
    let diff = match git.run(&[
        "diff",
        "--name-status",
        "-z",
        merge_base_sha.as_str(),
        head_sha.as_str(),
    ]) {
        Ok(bytes) => bytes,
        Err(err) => {
            return failed(format!(
                "cannot read `git diff {merge_base_sha} {head_sha}`: {err}"
            ));
        }
    };
    let changed = match parse_name_status(&diff) {
        Ok(files) => ChangedFiles::new(files),
        Err(err) => return failed(err),
    };
    Discovery {
        range: Some(ResolvedRange {
            requested: reference.to_owned(),
            base_sha,
            head_sha,
            merge_base_sha,
        }),
        changed,
    }
}

/// Parse `git diff --name-status -z` output.
///
/// Records are NUL-separated: `<status> NUL <path>` normally, and
/// `<status> NUL <from> NUL <to>` for `R`/`C`. A rename yields the destination
/// as [`ChangeKind::Renamed`] *and* the source as a [`ChangeKind::Deleted`]
/// entry, so a rule matching either side of the move triggers.
pub fn parse_name_status(bytes: &[u8]) -> Result<Vec<ChangedFile>, String> {
    let tokens = bytes
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let status = std::str::from_utf8(tokens[index]).map_err(|_| {
            format!("non-UTF-8 status in `git diff --name-status -z` at record {index}")
        })?;
        index += 1;
        let is_move = status.starts_with('R') || status.starts_with('C');
        if is_move {
            if index + 1 >= tokens.len() {
                return Err(format!(
                    "malformed `git diff --name-status -z`: status {status} without a source and destination path"
                ));
            }
            let from = utf8_path(tokens[index], index)?;
            let to = utf8_path(tokens[index + 1], index + 1)?;
            index += 2;
            files.push(ChangedFile {
                path: from.clone(),
                kind: ChangeKind::Deleted,
            });
            files.push(ChangedFile {
                path: to,
                kind: ChangeKind::Renamed { from },
            });
        } else {
            if index >= tokens.len() {
                return Err(format!(
                    "malformed `git diff --name-status -z`: status {status} without a path"
                ));
            }
            let path = utf8_path(tokens[index], index)?;
            index += 1;
            files.push(ChangedFile {
                path,
                kind: kind_for(status),
            });
        }
    }
    Ok(files)
}

fn utf8_path(token: &[u8], index: usize) -> Result<String, String> {
    std::str::from_utf8(token)
        .map(str::to_owned)
        .map_err(|_| format!("non-UTF-8 path in `git diff --name-status -z` at record {index}"))
}

fn kind_for(status: &str) -> ChangeKind {
    match status.chars().next() {
        Some('A') => ChangeKind::Added,
        Some('D') => ChangeKind::Deleted,
        // 'M' modified, 'T' type change, 'U' unmerged, and anything Git adds
        // later: treat as a content change rather than dropping the path.
        _ => ChangeKind::Modified,
    }
}
