---
name: github-ci-guardrails
description: Validate merge readiness with atomic commits and GitHub Actions checks using gh CLI; use for pre-merge verification, CI truth validation, and ruleset merge-state gating (required contexts, CodeQL code_scanning, mergeStateStatus).
---

# GitHub CI Guardrails

1. Ensure change set is one logical unit (atomic commit).
2. Run local gates from `references/local-gates.md`.
3. **Commitlint full PR range** before push:
   `npx commitlint --from origin/main --to HEAD --verbose`
   Read the **aggregate** verdict, not the last per-commit `✔` — every commit prints its own block, so
   `tail -8` shows one clean commit while an earlier one is red (measured 2026-10-29, on my own
   branch: `footer-max-line-length` at 103 chars had already failed locally and I never saw it).
   Grep the whole output for `✖`. `footer-max-line-length` judges the *last paragraph* no matter how
   prose-y it is; wrap bodies under 100.
   A red range is fixed by **rewording** the offending commit, not by a follow-up commit — the range
   re-checks every message, so the bad line stays red: backup branch, `git reset --hard <parent>`,
   `git cherry-pick --no-commit <bad>` + `git commit -F <msg>`, cherry-pick the rest, then
   `git push --force-with-lease`.
   This is a **per-push** gate, not a per-PR one: re-run it after every amend and after each of the
   two rewrites, never only on the first push (measured 2026-10-30 — a 101-char body line reached an
   open PR because the lint was run on commit 1 and not on commit 4, and its own PR body then asserted
   rc 0). Nothing in CI's visible surface warns you in the meantime: `Tooling Guard` had already passed
   while the job that owns `commitlint` sat `queued` for 25 min, so the failing check was invisible, and
   a re-push cancelled it before it could speak. Measure the exit status directly: piping to `tail`
   and then reading `$?` reports the pipe's status, not the linter's.
   **The linted string and the committed string differ after a squash.** GitHub builds the subject as
   `<PR title> (#<NNN>)`, so the budget is `100 - len(" (#<NNN>)")` (7 for 3-digit, 8 for 4-digit), and
   branch-commit linting can never see it (issue #965). Measured 2026-10-30: #947's title was 97 —
   legal — and its squash subject `…bounded OTLP (#947)` was **104**, failing `header-max-length` on
   `main` (run 41059071360, job `commitlint`, check-run 124073904392). Check before merging:
   `printf '%s (#%s)' "$(gh pr view $N --json title -q .title)" "$N" | wc -c`. The red self-heals only on
   the *next* merge (the push range then starts at the bad commit and drops it), so main can sit red and
   refuse every dependent `workflow_run` — release and pre-release gate included.
4. Validate GitHub checks with `references/gh-ci-truth.md`.
5. If checks fail, report failing job and log URL; map failure class via
   `references/ci-pitfalls-pr-triage.md` (commitlint scope, mutation surface,
   clippy `duplicated_attributes`, Jules force-push regression).
6. Do not claim green until `gh pr checks` shows no failures.

## Mutation gate (fast in-diff)

- Score = killed / (killed + missed); threshold typically 85% (`MUTATION_THRESHOLD`).
- **Minimize unrelated churn** in the PR diff — cosmetic rewrites of adjacent
  functions put them in the mutant set (e.g. `import_json -> Ok(1)` survives a
  test that only imports one concept).
- **Kill comparison boundary mutants** when using `select_nth_unstable_by(k)`:
  test the `len == k` path so `>` → `>=` panics and is caught.
- **CLI async entry points** (`run_query -> Ok(())`) are usually unkillable under
  `--lib`-only mutation CI; prefer exclude in `scripts/mutation_test.sh` with a
  one-line rationale, not path-excluding whole production modules.
- **A low score can be a denominator artifact, not a test gap**: cfg-blind **phantom mutants**
  (`#[cfg(target_arch = "wasm32")]` or an off feature) are lines the profile never compiles, so no
  test can kill them. Count them out with evidence before believing the number — #947 moved
  **62% → 94.0476%** when 19 wasm32 phantoms left the set. Never chase one with a test.
  `references/ci-pitfalls-pr-triage.md`.

## Pre-merge re-check (multi-PR or bot PRs)

```bash
git fetch origin main
git diff --stat origin/main...HEAD   # unexpected reverts?
npx commitlint --from origin/main --to HEAD --verbose
gh pr checks <n>
```

### After a bot force-push or a bot merge

Re-derive the **tree**, not just the head SHA: `git diff --stat origin/main...HEAD` shows the PR's
intended files, and `git diff origin/main HEAD -- <dirs main recently touched>` catches a merge
resolution that silently restored an old side of a conflict (measured 2026-10-10, #861: main's
`query_count` implementation had vanished from a branch whose own commits never touched the file,
while every check stayed green). `--force-with-lease` detects remote movement, never content loss;
rebuilding as `main` + the intended files is safer than trusting the merge.

### Perf claims in a PR body

- Recompute the percentage the body prints from its own numbers before judging the claim
  (measured 2026-10-10, #871: `657.37 → 648.80 µs` labelled "~3.8%"; it is −1.3%).
- A single A/B pair on a shared host proves nothing. Run A/B/A′ and require the sign to repeat:
  #871's medians fell `1.80 → 1.42 → 1.33 ms` *across all three runs regardless of code* — drift,
  not the diff — while criterion's paired p moved from 0.58 (A→B) to <0.01 the other way (B→A′).

## What actually gates a merge (ruleset, not the check list)

Green checks + `MERGEABLE` can still read `mergeStateStatus: BLOCKED`: the branch ruleset gates too, and
its non-status rules are invisible to `gh pr checks` — this repo's ruleset also carries `code_scanning`
+ `code_quality`, so a pending CodeQL analysis holds `BLOCKED` after every required context is green
(2026-10-17, #909). `gh pr checks --watch` is not a reliable long poll (it died mid-run on a TLS
handshake timeout; exit 1 = client error, not a verdict) — snapshot with `--json name,bucket` or the
check-runs API in a bounded loop. Recipes and the measured state: `references/gh-ci-truth.md`.

## An open PR blocks the *release*, not just the merge (2026-10-31)

`pre-release-gate.yml`'s `pr-drain-check` job (`:37-50`) runs `gh pr list --state open` and `exit 1` unless
**every** open PR carries `hold-for-next` or `deferred`. Since `release.yml` refuses any main head whose gate
run is not `completed/success`, one open PR blocks every release — a `recover=true` dispatch included.
Measured: run 41188203137 (`bfa15d49`) refused with `2 of 7 gate jobs failed: pr-drain-check, gate-summary`
and created no tag. So draining or labelling the queue is a **step of the release dispatch**, not
housekeeping, and a release that never started is not evidence of a quality failure.
Label *existence* is a measurement, not an assumption: `gh api repos/<o>/<r>/labels` paginates at 30 of this
repo's 53, so `hold-for-next` read as absent until re-queried with `?per_page=100`. Additive labelling is
`gh api -X POST repos/<o>/<r>/issues/<n>/labels -f labels[]=hold-for-next` (`PUT` replaces the set;
`gh pr edit --add-label` hits the known `projectCards` failure). Annotation-fetch shape + the drain sequence:
`references/gh-ci-truth.md`.

## Event payloads are point-in-time

`pull_request` runs fire on open/synchronize/reopen: editing the title or body triggers **no** run, and
commitlint plus the ADR-0095 evidence gate read the values captured in the *run's payload*. Set the
title and the body before the push that starts the run — or refresh the head afterwards (identical
amend + force-push) so the new payload carries them (2026-10-10 #869 title; 2026-10-17 #909 body).

## `workflow_run` gate conclusions

`workflow_run` cannot filter on the upstream conclusion. In downstream `always()` summary jobs,
represent a non-success upstream run as **no verdict**; do not infer a failed verdict from skipped
dependent jobs. When Cargo Audit and Cargo Deny consume separate RustSec policy files, gate ignore-ID
parity with a fail-closed checker. Sources: `.github/workflows/pre-release-gate.yml`,
`scripts/pre-release-gate-summary.sh`, and `scripts/check-audit-ignore-parity.sh`.

### Two workflows on one event cannot read each other's verdicts (2026-10-30, #953/#962)

`release.yml` and `pre-release-gate.yml` both trigger on CI completion, so a *one-shot* gate read inside
release `validate` samples a sibling that is ~1 min into a ~4½-min run (`validate` measured 62 s, slowest
gate pass 4m37s). Bound a wait and fail loudly rather than reading once — and prefer loudness over the
event-driven re-key when the silent alternative is "no run at all": 69 consecutive release runs had
**zero jobs** (GitHub rejecting double-quoted literals in `if:`) for eight days with nothing surfacing it.
`total_count == 0` on `…/runs/<id>/jobs` is the tell; `scripts/check-actionlint.sh` is the gate.

Before settling any such design objection, grep `progress/LEARNINGS.md` and `plans/PR_ROAST_*.md` for the
**rule**, not just the symbol: `Polling a gate is a ceiling you will raise forever; migrate to the event`
was recorded on 2026-09-30, and #962 re-derived the same tension in isolation and answered it without ever
citing it. A change that departs from a standing rule says so in the code comment and the PR body, with the
measured reason and the condition that would overturn it.

## A green `cargo deny` says nothing about a GHSA that has no RustSec id (2026-10-30, #966)

`cargo deny`/`cargo audit` read **RustSec**; Dependabot reads the **GitHub advisory database**. The sets
overlap but neither contains the other, so a *critical* Dependabot alert can coexist with every security gate
green and with a correct `deny.toml` — there is simply no id to ignore or to fail on. Measured case:
`GHSA-m665-v7pm-7grj`/CVE-2026-115905 (zip `<= 9.0.4`, patched 9.0.5, published 2026-10-28) against a lock at
`zip 6.0.0`, while the three RustSec-visible zip advisories (`RUSTSEC-2026-0367/0368/0369`) were already
ignored on a verified mechanism claim. Consequences for triage: (1) treat the push-time
"GitHub found N vulnerabilities" banner as a signal to query `…/dependabot/alerts?state=open`, not as a
duplicate of the gate; (2) never argue safety from `Cargo.lock` — it cannot express dependency **kind**, so
build-vs-runtime reachability comes from the sparse index (`"kind":"build"`) or the vendored manifest; (3) when
an ignore's reason is a feature flag, make the flag a checked assumption, because the ignore is only as durable
as a `Cargo.toml` line nothing reads.
