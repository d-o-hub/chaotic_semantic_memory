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

### Scope enum (must match `commitlint.config.cjs`)

Common scopes: `framework`, `persistence`, `retrieval`, `cli`, `wasm`, `ci`,
`docs`, `memory`, `core`, `workspace`, `plans`, `goap`, `agents`.

- Prefer **lowercase subject** start: `fix(ci): repair wasm out-dir` — not
  `fix(ci): Repair WASM` / `feat(framework): TTL lifecycle` (subject-case fails).
- **Validate the full PR range before push** (single-commit local checks miss
  earlier bad commits on the branch):

```bash
npx commitlint --from origin/main --to HEAD --verbose
```

If amend/reword is needed on a shared branch, use `git push --force-with-lease`
only after confirming no one else has based work on the tip.

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
npx commitlint --from origin/main --to HEAD --verbose   # full range
gh pr checks --watch
gh run list --branch <branch> --limit 5
gh run view <run-id> --log-failed   # on failure: root-cause before retry
```

Do not claim success until both local and GitHub checks pass.

### Recurring CI failure classes (prevent)

| Failure | Prevention |
|---------|------------|
| commitlint scope-enum | Use scopes from `commitlint.config.cjs`; add new scopes when introducing crates/plan areas |
| commitlint subject-case | Lowercase first word of subject (`ttl` not `TTL`) |
| wasm smoke / missing js | Absolute `--out-dir` for `wasm-pack build crates/csm-wasm` |
| macos-arm64 unreachable_code | `#[cfg(not(target_arch = "aarch64"))]` after NEON early return |
| mutation timeouts on stubs | Exclude or test feature-disabled stubs; timeouts do not inflate score |
| skill catalog stale | `./scripts/generate-skill-catalog.sh` then `--check` before commit |
| cargo-fuzz short run / musl | version from `.github/ci-settings.env`; `--sanitizer none` on PR |

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
