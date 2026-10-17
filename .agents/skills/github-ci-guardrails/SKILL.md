---
name: github-ci-guardrails
description: Validate merge readiness with atomic commits and GitHub Actions checks using gh CLI; use for pre-merge verification, CI truth validation, and ruleset merge-state gating (required contexts, CodeQL code_scanning, mergeStateStatus).
---

# GitHub CI Guardrails

1. Ensure change set is one logical unit (atomic commit).
2. Run local gates from `references/local-gates.md`.
3. **Commitlint full PR range** before push:
   `npx commitlint --from origin/main --to HEAD --verbose`
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

## Event payloads are point-in-time

`pull_request` runs fire on open/synchronize/reopen: editing the title or body triggers **no** run, and
commitlint plus the ADR-0095 evidence gate read the values captured in the *run's payload*. Set the
title and the body before the push that starts the run — or refresh the head afterwards (identical
amend + force-push) so the new payload carries them (2026-10-10 #869 title; 2026-10-17 #909 body).
