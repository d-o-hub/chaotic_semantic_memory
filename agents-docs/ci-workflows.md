# CI Workflow Map

Every workflow in `.github/workflows/`, what fires it, and what it gates —
so an agent never has to re-derive the pipeline from YAML (issue #1069).
AGENTS.md names `ci.yml` (the sprint rules); `agents-docs/release-safety.md`
owns the release trio (`release.yml` / `pre-release-gate.yml` /
`publish-crates.yml`); this map covers the rest and the required-context list.

## Workflows

| Workflow | Fires on | What it does / gates |
|---|---|---|
| `ci.yml` | `push` to `main` + every PR | The main gate. `detect-changes` runs `csm-ci-select` (policy globs live in `crates/csm-ci-selection`, never as workflow regexes) and enables/disables jobs per path class, failing closed to every job on ambiguity. Owns the required merge contexts: `lint`, `test`, `mutation-test` (8-shard union), `commitlint`, `Build CLI (linux-x64)`, plus Codacy (see below). |
| `tooling-guard.yml` | PRs against `main` | Guards critical-file deletions (`.githooks/pre-commit`, `ci.yml`, `scripts/harness-check.sh`, `scripts/quality-gates.sh`, `flake.nix`, …) and runs lightweight lint fixtures; independent of `ci.yml`'s path selection. |
| `benchmark-ci.yml` | PRs touching benchmark paths | Runs the graph-candidates benchmark and `scripts/benchmark-evidence.sh` for scale evidence (ADR-0095 evidence tiers). |
| `fuzz.yml` | PRs touching fuzz paths | `fuzz-short`: ~30s runs on changed targets; weekly cron covers the full sweep. |
| `version-integrity.yml` | PRs touching version surfaces | Keeps version literals synchronized across manifests/docs (`scripts/check-version-literals.py`). |
| `dependabot-auto-merge.yml` | Schedule (`*/15`) | Auto-merges *roasted* Dependabot PRs once every required context is green on their current head (#1061); it never sweeps non-Dependabot PRs. |
| `scheduled-evidence.yml` | Weekly cron (Sunday) | Scale evidence + full mutation sweep (weekly evidence tier). |
| `pages.yml` | `push` to `main` | Publishes the mdBook (`book/`) to GitHub Pages. |
| `dist.yml` | `workflow_dispatch` | Manual dist-channel build dispatch (see `dist-channel-selection` skill). |
| `auto-release.yml` | `workflow_run` on CI completion (`main`) + `workflow_dispatch` | Release preparation and tagging: `prepare` derives the bump from conventional commits (`scripts/release-derive.sh`), opens/refreshes the `release/vX.Y.Z` PR and retires superseded proposals; `ship` (dispatch or next green `main` CI) tags the `main` tip and dispatches `release.yml -f tag=v*`. See `agents-docs/release-safety.md`. |
| `release.yml` | `v*` tag push + `workflow_dispatch` (tag = release/recovery, empty = dry run) | Validates the tagged commit (tag == version, CI + Pre-Release Gate green for that SHA) → build matrix + provenance → GitHub Release → dispatches `publish-crates.yml` → npm publishes → `verify-release`. Never creates the tag. See `agents-docs/release-safety.md` and ADR-0042. |
| `pre-release-gate.yml` | `workflow_run` on CI completion | The drain/refusal gate: refuses to release while unlabelled PRs are open; bounded wait, no `--auto` double-trigger. See the 2026-11-25 roast record and `github-ci-guardrails` skill. |
| `publish-crates.yml` | `workflow_dispatch` | ADR-0104 OIDC crates.io publishing (30-min token; the trusted-publisher configs must name *this* workflow file). |

## Required merge contexts — and the one that is not in this tree

The branch ruleset gates merges on: `lint`, `test`, `mutation-test`,
`commitlint`, `Build CLI (linux-x64)`, and `Codacy Static Code Analysis`, with
non-status rules holding `mergeStateStatus: BLOCKED` for ~18 min after contexts
go green (CodeQL `code_scanning`/`code_quality` — wait it out, it is never a
red).

**`Codacy Static Code Analysis` is produced by the external Codacy app, not by
anything in this repository.** There is no workflow, config, or script here
that creates it, and nothing in-tree can fix it directly: quality-gate
failures and false-positive suppressions are worked through the `codacy`
skill, and the context is expected to appear on every PR automatically. Do
not chase it in-repo (do not add workflows, do not edit the check list trying
to create it) — it is visible only on the PR's check list and ruleset.
