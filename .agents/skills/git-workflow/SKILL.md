---
name: git-workflow
description: Git commit conventions, validation gates, and CI/CD workflows. Use when committing changes, verifying gates, or working with GitHub Actions.
---

# Git Workflow

## Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <short summary in imperative mood>

<body: explain what and why, not how>

<footer: BREAKING CHANGE, Co-authored-by, etc.>
```

See `references/commit-types.md` for full type/scope guide and examples.

## Validation Gates (Pre-Commit)

Run before every commit:

```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo check --message-format=short
cargo test --all-features --quiet
cargo fmt --check
cargo clippy -- -D warnings
```

Or use: `scripts/validate.sh`

## Performance Gate

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```

Target: `reservoir_step_50k < 100μs`

## CI Verification

```bash
gh pr checks --watch
gh run list --branch <branch> --limit 5
```

Do not claim success until both local and GitHub checks pass.
