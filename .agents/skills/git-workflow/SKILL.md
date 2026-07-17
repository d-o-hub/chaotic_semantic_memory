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

## Review Rule
- Block commits that introduce hardcoded runtime settings or unexplained numeric literals in tunable paths.

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

## PR Triage Checklist (Multi-PR Review)

When reviewing/fixing multiple open PRs, **always check ALL of these for EVERY PR** before starting work:

```bash
# 1. Merge conflict check (FIRST - before anything else)
gh pr list --state open --json number,title,mergeable \
  --jq '.[] | "\(.number): \(.title) — \(.mergeable)"'

# 2. CI status check
for pr in $(gh pr list --state open --json number --jq '.[].number'); do
  echo "PR #$pr:"
  gh pr checks $pr 2>&1 | grep "fail" | wc -l
  echo "failures"
done

# 3. Review comments requiring action
gh pr list --state open --json number,reviewDecision \
  --jq '.[] | "\(.number): \(.reviewDecision)"'
```

**Merge order rules:**
1. Check `mergeable` status FIRST — resolve conflicts before CI fixes
2. Independent PRs (docs, CI-only) merge first
3. Foundation PRs (lints, commitlint config) merge before dependent PRs
4. Feature PRs that touch the same files go last
5. After each merge, rebase remaining PRs on updated main

**Never skip:** A PR showing `MERGEABLE` in the API can still have conflicts after other PRs merge. Re-check after each merge.

## Merging Multiple PRs (Sequential Merge Protocol)

**⚠️ NEVER use `--auto` merge when the repo requires "up to date with base branch".**

This repo requires PRs to be up-to-date with main before merge. Auto-merge
creates an infinite loop:
1. Set auto-merge on PR A, B, C
2. PR A merges → main moves
3. PR B is now stale → auto-merge cancelled
4. Must rebase B → new CI run → wait → repeat

**Correct protocol — merge one at a time:**

```bash
# For each PR in dependency order:
git checkout <branch>
git fetch origin main
git rebase origin/main
git push origin <branch> --force-with-lease

# Wait for CI to pass (~15 min)
gh pr checks <number> --watch

# Merge (only after ALL checks green)
gh pr merge <number> --squash --delete-branch

# Then move to next PR (main has changed)
```

**Use `scripts/pr-triage.sh`** to get the full picture before starting.

**Anti-patterns to avoid:**
- ❌ `gh pr merge --auto` on multiple PRs (creates rebase loops)
- ❌ Rebasing all PRs at once then hoping they all pass CI simultaneously
- ❌ Merging without waiting for ALL CI checks (not just "critical" ones)
- ❌ Skipping merge conflict check before starting CI fixes
