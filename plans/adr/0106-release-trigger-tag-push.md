# ADR-0106: Release Trigger Moves to Tag Push With an Automated Bump Proposal

## Status

Accepted (2026-11-27). Supersedes the trigger model and tag-ownership section of
[ADR-0042](0042-release-automation-v010.md); ADR-0042's same-SHA gate requirement, and
crates.io publishing from a dispatch workflow ([ADR-0104](0104-crates-io-oidc-trusted-publishing.md)),
are retained unchanged.

## Context

ADR-0042 wired the release to CI completion: `release.yml` was `workflow_run`-triggered, resolved that
run's `head_sha`, verified the exact SHA against `pre-release-gate.yml`, and then **created the
`vX.Y.Z` tag itself** before packaging. Three costs came with that, each measured:

1. **Tag creation and publication were the same operation**, so recovery and retry were entangled. The
   `no-verdict` job existed only to describe the case where a run concluded green having published
   nothing (run 41186134783, 2026-10-31: nine jobs skipped, crates.io still at 0/9). A workflow whose
   green state needs a narrator is a workflow whose trigger is wrong.
2. **"Never tag by hand"** was the correct rule under that model, which meant the only operator recovery
   path was also the forbidden one.
3. **The bump was a human edit** of `Cargo.toml` + `CHANGELOG.md` with no derivation, so the version
   number was a judgement made under time pressure, and the workspace had no mechanical answer to
   "what does the next release deserve?".

## Decision

Split the three concerns that ADR-0042 fused:

- **`auto-release.yml` owns the tag.** After a green `CI` push to `main`, `prepare` derives the SemVer
  bump from conventional commits since the last release tag (`scripts/release-derive.sh`: batch
  threshold of three release-worthy commits, `feat!`/`BREAKING CHANGE` → major at ≥1.0 else minor,
  `feat` → minor, `fix`/`perf` → patch, non-conventional → patch, `Rename-Confirmed:` trailer,
  unconfirmed shipped-surface rename → `held-rename`) and opens or refreshes a single
  `release/vX.Y.Z` PR (`sync-version.sh` + changelog section + `verify-version-sync.sh`), closing a
  proposal the derivation no longer produces. After a maintainer merges, `ship` tags the verified
  `main` tip and dispatches `release.yml`.
- **`release.yml` is triggered by the tag**, not by CI completion: `v*` tag push, tagged
  `workflow_dispatch` (idempotent recovery, `-f tag=vX.Y.Z`), or untagged `workflow_dispatch` (dry
  run: builds and packages everything, publishes nothing). It never creates a tag.
- **Publication stays where ADR-0104 put it** — `publish-crates.yml` via `trigger-crates-io-publish`,
  because the nine crates.io trusted-publisher configs name that workflow filename.

A `GITHUB_TOKEN` tag push starts no `push` workflow, so `ship`'s explicit `gh workflow run release.yml`
dispatch is load-bearing, not cosmetic. The same limitation means a `GITHUB_TOKEN`-pushed release PR
does not run its own checks: `gh workflow run ci.yml -f ref=release/vX.Y.Z` is required, and a
maintainer merges it.

## Guards retained (fail-closed by construction)

- `ship` tags only when `release-derive.sh next --published` reports `pending`; it asserts
  `EXPECTED_SHA == HEAD`, refuses a tag that already has a GitHub Release, and recovery-dispatches
  instead of re-tagging.
- `release.yml`'s `validate` still requires the newest **`CI` and `Pre-Release Gate`** runs for the
  tagged SHA to be `completed/success` (bounded 30-minute poll), asserts tag == root
  `[package].version`, and refuses untagged publication.
- The dry run is a distinct trigger rather than a flag, so "published nothing" cannot be mistaken for
  a failed release.

## Consequences

Positive: tag creation has one owner and one trigger (a human merge); recovery is `-f tag=…` against
an existing tag; a green release run can no longer be green having published nothing, because the
dry run is a different trigger and says so.

Accepted costs: `Cargo.toml` and the derivation are two views of the version, mitigated by the
tag == version assertion in `validate`; a release PR needs its checks started by hand; an automated
bump means a merge can publish without anyone typing a version, which is why the merge — not the
derivation — is the approval gate; and `release-derive.sh` becomes a 571-line shell dependency of the
release path, tested by its own self-test rather than by the Rust suite.

## Evidence at acceptance (2026-11-27, PR #1089 head)

- `scripts/test-verify-release-step.sh` — 137 cases, 0 failures (structural + replay against the new
  `release.yml` and the untouched `publish-crates.yml`)
- `scripts/test-release-verdict.sh` — 29 assertions, 0 failures (13 cases; the four `no-verdict` cases
  were deleted with the script when the `no-verdict` job was retired)
- `scripts/release-derive.sh --self-test` — pass, including rename-hold and tagged-but-unpublished
- `scripts/validate-workflows.sh --check` — 0 errors, advisory baseline identical to `main`
- `scripts/validate.sh` — rc 0 on the PR head
