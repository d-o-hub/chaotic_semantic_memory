---
name: github-ci-guardrails
description: Validate merge readiness with atomic commits and GitHub Actions checks using gh CLI; use for pre-merge verification and CI truth validation.
---

# GitHub CI Guardrails

1. Ensure change set is one logical unit (atomic commit).
2. Run local gates: `scripts/ci-preflight.sh` (fast) or `scripts/validate.sh` (full).
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
