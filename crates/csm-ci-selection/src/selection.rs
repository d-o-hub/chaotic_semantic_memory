//! Mapping a change set onto job selection, deterministically.
//!
//! Same policy + same change set + same mode ⇒ same selected/skipped split with
//! the same reasons, every time. Reasons are data ([`Selected::reason`]), not
//! prose assembled at the call site, so a CI log answers "why did this run?" and
//! "why did that not?" without re-deriving the rules by hand.

use crate::changed::ChangedFiles;
use crate::policy::{Job, JobMatch, Policy, TrunkPolicy};

/// Discovery failed, so nothing is known about the change set.
pub const FAIL_CLOSED_REASON: &str = "change discovery failed; every job selected (fail-closed)";
/// The change set is empty and a successful empty diff is still no information.
pub const NO_CHANGE_INFORMATION_REASON: &str =
    "empty change set; every job selected (no information)";
/// A path matched no rule at all, so no job can claim it is irrelevant.
pub const UNCLASSIFIED_REASON: &str = "unclassified path; every job selected (conservative)";
/// A push to main always protects this job.
pub const TRUNK_ALWAYS_REASON: &str = "push to main: trunk coverage";
/// A push to main never runs this job (pull-request-only evidence).
pub const TRUNK_NEVER_REASON: &str = "push to main: pull-request-only job";
/// No pattern of this job matched any changed path.
pub const NO_MATCH_REASON: &str = "no configured path matched";
/// A caller with no change set to classify asked for every job (`--all`).
pub const ALL_JOBS_REASON: &str = "--all: every job selected by request";

/// Which event is being classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A pull request: path-filtered, per [`crate::policy::JOBS`].
    PullRequest,
    /// A push to main: trunk policy per job.
    Trunk,
}

/// A job that will run, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    /// The `detect-changes` output key.
    pub name: String,
    /// Why it was selected.
    pub reason: String,
}

/// A job that will be skipped, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The `detect-changes` output key.
    pub name: String,
    /// Why it was skipped.
    pub reason: String,
}

/// The split, plus any note that explains a non-path decision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    selected: Vec<Selected>,
    skipped: Vec<Skipped>,
    notes: Vec<String>,
}

impl Selection {
    /// The jobs that will run.
    #[must_use]
    pub fn selected(&self) -> &[Selected] {
        &self.selected
    }

    /// The jobs that will be skipped.
    #[must_use]
    pub fn skipped(&self) -> &[Skipped] {
        &self.skipped
    }

    /// Diagnostics: a failed discovery, the unclassified paths, an empty set.
    #[must_use]
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// Whether `name` was selected.
    #[must_use]
    pub fn is_selected(&self, name: &str) -> bool {
        self.selected.iter().any(|entry| entry.name == name)
    }

    /// Why `name` was selected or skipped, if it is part of the policy.
    #[must_use]
    pub fn reason(&self, name: &str) -> Option<&str> {
        self.selected
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.reason.as_str())
            .or_else(|| {
                self.skipped
                    .iter()
                    .find(|entry| entry.name == name)
                    .map(|entry| entry.reason.as_str())
            })
    }

    /// The output keys in policy order: `true` selects, `false` skips.
    #[must_use]
    pub fn outputs(&self, policy: &Policy) -> Vec<(String, bool)> {
        policy
            .jobs()
            .iter()
            .map(|job| (job.name().to_owned(), self.is_selected(job.name())))
            .collect()
    }
}

/// Classify a change set for `mode`.
///
/// Fail-closed order: a failed discovery selects every job, an empty change set
/// selects every job, and (on a pull request) any unclassified path selects
/// every job. Only a non-empty, fully classified change set can skip a job.
#[must_use]
pub fn select(policy: &Policy, changed: &ChangedFiles, mode: Mode) -> Selection {
    if changed.is_failed() {
        let diagnostic = changed
            .error()
            .map_or_else(|| "no diagnostic".to_owned(), str::to_owned);
        return select_all(
            policy,
            FAIL_CLOSED_REASON,
            format!("{FAIL_CLOSED_REASON}: {diagnostic}"),
        );
    }
    let paths = changed.paths();
    if paths.is_empty() {
        return select_all(
            policy,
            NO_CHANGE_INFORMATION_REASON,
            NO_CHANGE_INFORMATION_REASON.to_owned(),
        );
    }
    if mode == Mode::PullRequest {
        let unclassified = paths
            .iter()
            .copied()
            .filter(|path| !is_classified(policy, path))
            .collect::<Vec<_>>();
        if !unclassified.is_empty() {
            return select_all(
                policy,
                UNCLASSIFIED_REASON,
                format!("{UNCLASSIFIED_REASON}: {}", unclassified.join(", ")),
            );
        }
    }

    let mut selection = Selection::default();
    for job in policy.jobs() {
        let outcome = match (mode, job.trunk()) {
            (Mode::Trunk, TrunkPolicy::Always) => Outcome::Selected(TRUNK_ALWAYS_REASON.to_owned()),
            (Mode::Trunk, TrunkPolicy::Never) => Outcome::Skipped(TRUNK_NEVER_REASON.to_owned()),
            _ => by_paths(job, &paths),
        };
        match outcome {
            Outcome::Selected(reason) => selection.selected.push(Selected {
                name: job.name().to_owned(),
                reason,
            }),
            Outcome::Skipped(reason) => selection.skipped.push(Skipped {
                name: job.name().to_owned(),
                reason,
            }),
        }
    }
    selection
}

/// Every job selected for the same reason, with `note` explaining the decision.
///
/// Public because a caller can know the change set is unusable before discovery
/// (the workflow's first-push case passes `--all`): the "run everything" split
/// must come from one place, not be re-implemented at the call site.
#[must_use]
pub fn select_all(policy: &Policy, reason: &str, note: String) -> Selection {
    let mut selection = Selection {
        selected: Vec::with_capacity(policy.jobs().len()),
        skipped: Vec::new(),
        notes: vec![note],
    };
    for job in policy.jobs() {
        selection.selected.push(Selected {
            name: job.name().to_owned(),
            reason: reason.to_owned(),
        });
    }
    selection
}

/// A path is classified when it is declared non-build, or when some job's rule
/// matches it. Everything else selects every job: a rule set that has never
/// seen a path cannot claim the path is harmless.
fn is_classified(policy: &Policy, path: &str) -> bool {
    policy.is_non_build(path)
        || policy
            .jobs()
            .iter()
            .any(|job| matches!(job.classify(path), JobMatch::Matched { .. }))
}

enum Outcome {
    Selected(String),
    Skipped(String),
}

/// The job's path rule: the first changed path that its include accepts and its
/// exclusions keep selects it. A path an exclusion rejects records the reason
/// but does not veto another path that matched.
fn by_paths(job: &Job, paths: &[&str]) -> Outcome {
    let mut excluded: Option<String> = None;
    for path in paths {
        match job.classify(path) {
            JobMatch::Matched { pattern } => {
                return Outcome::Selected(format!("matched `{pattern}`"));
            }
            JobMatch::Excluded { pattern } => {
                if excluded.is_none() {
                    excluded = Some(pattern);
                }
            }
            JobMatch::NoMatch => {}
        }
    }
    Outcome::Skipped(match excluded {
        Some(pattern) => format!("excluded by `{pattern}`"),
        None => NO_MATCH_REASON.to_owned(),
    })
}
