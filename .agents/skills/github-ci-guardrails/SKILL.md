---
name: github-ci-guardrails
description: Validate merge readiness with atomic commits and GitHub Actions checks using gh CLI; use for pre-merge verification and CI truth validation.
---

# GitHub CI Guardrails

1. Ensure change set is one logical unit (atomic commit).
2. Run local gates from `references/local-gates.md`.
3. Validate GitHub checks with `references/gh-ci-truth.md`.
4. If checks fail, report failing job and log URL.
