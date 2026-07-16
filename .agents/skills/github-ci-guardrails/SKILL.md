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
   `references/ci-pitfalls-wave32.md` (wasm out-dir, aarch64 unreachable,
   mutation stubs, skill catalog, TTL shutdown, absence short-circuit).
6. Do not claim green until `gh pr checks` shows no failures.
