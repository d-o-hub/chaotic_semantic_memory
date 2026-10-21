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

## Branch and Validation Gates

- Never commit or push directly on `main`; create or use a task branch first.
- Stage only the intended paths. Never use `git add -A` in a dirty worktree.
- On a fresh checkout, run `scripts/install-hooks.sh` before validation.
- Before every commit, run both required repository gates:

```bash
scripts/validate.sh
scripts/harness-check.sh all
```

- If `Cargo.toml` or `Cargo.lock` changes, also run `cargo deny --locked check`.
- Validate the full commit range with
  `npx commitlint --from origin/main --to HEAD --verbose`.
- `commitlint` applies to the entire branch range on every change; include it in the pre-commit checklist even when cargo tests and repository gates pass.

## Review Rule

- Block commits that introduce hardcoded runtime settings or unexplained numeric literals in tunable paths.

## Pull Request Handoff

- Roast the proposed PR diff using `skill://pr-roast-triage` before creating or
  merging a PR; record the verdict in `plans/PR_ROAST_<date>.md`.
- Confirm remote CI checks belong to the exact PR head SHA; a green badge or
  stale-head run is not evidence.
- Creating a PR does not authorize merging it. Merge only when directly asked,
  after a current-head CI pass and a recorded roast verdict.

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
2. Independent green PRs (docs, pure perf with all checks green) merge first
3. Foundation PRs (lints, commitlint config) merge before dependent PRs
4. Feature PRs that touch the same files go last
5. After each merge, rebase remaining PRs on updated main
6. Never `gh pr merge --auto` when clearing multiple PRs (rebase loop)

**Never skip:** A PR showing `MERGEABLE` in the API can still have conflicts after other PRs merge. Re-check after each merge.

## Jules / bot PR hygiene

- **Empty research PRs**: if `gh pr diff --name-only` is empty (0 files), close as
  no-op — do not spend mutation/CI budget.
- **Re-diff vs `origin/main` before merge**: Jules can force-push after a human/agent
  fix and silently drop sibling merges (e.g. remove Rayon from `probe_batch`).
  Always: `git diff --stat origin/main...HEAD` and skim for unexpected reverts.
- **Commitlint full range**: CI runs `commitlint --from base --to head`. One early
  bad scope (`perf(ops):`) fails even if later commits are valid. Squash/reword;
  validate with `npx commitlint --from origin/main --to HEAD --verbose`.
- **Invalid scopes are not inventable**: use only `commitlint.config.cjs` `scope-enum`
  (e.g. `framework`, not `ops`; `docs` without a fake `plans` scope).
- **Supplying the evidence a gate demands — on someone else's branch.** When the roast verdict is
  "correct but unmeasured", add what the gate needs yourself instead of stalling: the bench/comment
  commits (`test(<scope>): …`, `docs(<scope>): …`), the `## Performance Evidence` body section set
  *before* the push that starts the run, then merge from the verified head. Never retitle
  `perf(` → `refactor(` to make the gate skip — that is gate-gaming, not evidence.

## Landing a Wide Uncommitted Working Set

A 100+-path **staged index** with no commits (a carried-over working set) is not a branch yet, and the
instinct to rebase it onto a moved `main` is the wrong one. Protocol used for the 157-path audit wave:

1. **Split the index into scoped commits with `git commit --only -- <paths>`.** `--only` builds the
   commit from worktree content for *just* those paths and leaves the rest of the index staged, so one
   `git status` read of the whole set stays intact while each commitlint scope gets its own commit.
   Verify each group with `npx commitlint --from origin/main --to HEAD --verbose` at the end.
   (Never `git add -A`; the point is that the operator's index is consumed deliberately.)
2. **Merge `main` in once, do not rebase.** 11 commits had landed meanwhile
   (`git rev-list --count "$(git merge-base HEAD origin/main)..origin/main"`). A rebase replays every
   commit through the *generated* files (`llms.txt`, `llms-full.txt`, `Cargo.lock`, script/skill
   catalogs) and re-resolves each conflict it creates; one
   `git commit --only`-finished branch + a single `git merge --no-commit --no-ff origin/main` resolves
   them once. Regenerate generated files (`scripts/gen-llms-txt.sh`, `gen_script_catalog.py`) instead of
   hand-picking their hunks.
3. **Resolve per-hunk, never per-file, where both sides fixed the same region.** Both sides of this
   merge contained *landed* work: taking ours re-introduced the unchecked HNSW
   `unsafe { Self::as_hvec10240(..) }` cast `main` had removed; taking theirs dropped the wave's
   `searchable_text` metadata insert. The right answer was `main`'s structure with the branch's insert
   re-applied inside it. For every conflicted file ask: does either side contain something the other
   does not know about, and which side is `main`?
4. **Re-measure the counters after the merge, not before.** `docs/architecture/context.yaml`
   `total_tests`/`tests_passing` and `plans/GOAP_STATE.md` `tests_count` must agree with each other and
   `modules_under_loc_limit` must equal the live `src/**/*.rs` count
   (`tests/validate_llm_context.py:263`); a merge that adds test files moves all of them.

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

# Wait for CI to pass (~15 min). `--watch` can die on a TLS timeout: poll buckets instead.
gh pr checks <number> --json name,bucket --jq 'group_by(.bucket) | map("\(.[0].bucket)=\(length)") | join(" ")'

# The merge gate is the ruleset (required contexts + code_scanning/code_quality CodeQL runs):
gh pr view <number> --json mergeable,mergeStateStatus   # want MERGEABLE + CLEAN
gh pr merge <number> --squash --delete-branch

# Verify the landing, not just the command's exit code
git fetch origin main && git log --oneline -1 origin/main
git ls-remote origin 'refs/heads/<branch>' | wc -l      # 0 = branch deleted

# Then move to next PR (main has changed)
```

**Use `scripts/pr-triage.sh`** to get the full picture before starting.

**Anti-patterns to avoid:**
- ❌ `gh pr merge --auto` on multiple PRs (creates rebase loops)
- ❌ Rebasing all PRs at once then hoping they all pass CI simultaneously
- ❌ Merging without waiting for ALL CI checks (not just "critical" ones)
- ❌ Skipping merge conflict check before starting CI fixes
