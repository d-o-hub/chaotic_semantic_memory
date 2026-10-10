---
name: github-ci-guardrails
description: Validate merge readiness with atomic commits and GitHub Actions checks using gh CLI; use for pre-merge verification and CI truth validation.
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
