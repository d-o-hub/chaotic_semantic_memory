---
name: pr-roast-triage
description: Triage all open PRs — detect duplicates, verify CI truth, roast, and emit manual merge order. Use when reviewing multiple open PRs or clearing a bot-generated PR backlog.
---

# PR Roast Triage

Review every open PR, close what has no independent impact, roast the rest
with fix recommendations. The human merges manually in the emitted order —
never auto-merge, never merge from this skill.

## Step 1 — Triage baseline (run first, before any opinion)

```bash
./scripts/pr-triage.sh   # conflict status + CI status + suggested order
git status --short       # scope out unrelated local changes first
git diff HEAD --stat
```

## Step 2 — CI truth (annotations, not badges)

A red badge is not a verdict. Pull the failing step's annotation:

```bash
gh api repos/<owner>/<repo>/check-runs/<job_id>/annotations \
  --jq '.[].message | .[0:200]'
```

Known infra flakes (re-run, do not "fix code"):
- `Unable to authenticate to FlakeHub` → Cargo Deny nix setup broken, pre-existing.
- `hosted runner lost communication` → CodeQL/runner loss, pre-existing.
- External-only red (Codacy/Sonar) with all GitHub checks green → code issue, real.

## Step 3 — Duplicate detection (same base blob + same hunk)

Bot swarms file near-identical PRs. Compare, don't assume:

```bash
gh pr diff <a> --name-only
gh pr diff <b> 2>/dev/null | sed -n '1,80p'   # same base hash? same function?
```

Same file + same base blob + same effect = duplicates. Keep exactly one:
1. Green CI beats mergeable-but-red.
2. Fewest unrelated files (no `export.json` timestamp, no lockfile churn).
3. Backward-compat wrapper kept beats private-fn signature break.
4. Newest branch (rebases cleanest onto latest main).

Close the losers with reason `superseded by #<keeper>`.

## Step 4 — Roast rubric (every keeper PR)

- **Atomicity**: one logical change. Mega-PRs (>5 files across concerns) must split.
- **Title honesty**: title must describe the diff (`re-export` hiding a BFS cap = reject/retitle).
- **commitlint scope**: must exist in `commitlint.config.cjs` `scope-enum`
  (`perf(hyperdim)` fails; only listed scopes pass).
- **Rationale comments**: never deleted to "shorten" (`Never delete rationale comments`). Under 500 LOC gate pressure, bot PRs may attempt to strip docstrings/comments to make room for new code. Reject/roast this behavior; require extracting child submodules (e.g. `hyperdim_binary_serde.rs`) instead.
- **deny.toml**: must only ADD ignores with advisory ID + reason. Deleting or
  commenting out existing ignores re-breaks `cargo deny` — reject.
- **Perf claims**: no `criterion` output or flamegraph = not review-ready.
- **`export.json` / `Cargo.lock` noise**: timestamp-only or resolver-churn hunks
  must be dropped before merge.
- **Bot comments** (Jules hello, Sonar/Codacy pass notes) are noise, not reviews.
  Zero human comments = no hidden requirements, but also no approval.
- **GitHub CLI GraphQL deprecation**: `gh pr edit` and `gh issue view` fail if querying
  deprecated `projectCards`. Query specific `--json` fields for reads, or use
  `gh api -X PATCH repos/<owner>/<repo>/pulls/<id> -f title="..."` for editing PR titles.
- **Multi-issue parent linkage**: When a parent PR implements multiple child issues,
  ensure all child issues (`Fixes #A`, `Fixes #B`) are declared in the PR body so all
  issues close cleanly upon merge.

## Step 5 — Manual merge order (emit, do not execute)

1. Trivial green first (dependabot 1-liners).
2. Security/clamp fixes (small, high value).
3. Foundation before dependents (commitlint scope additions, owner/facade
   migrations before perf touches on the same files).
4. One keeper per duplicate cluster; closes reference the keeper.
5. Mega-PRs and SIMD/`unsafe` last (need evidence + rebase after everything).
6. After each merge: rebase next, re-run CI, re-check `mergeable`.

```bash
# per PR, in order:
git fetch origin main
gh pr checks <n>                       # all green?
git diff --stat origin/main...<branch> # no surprise reverts?
gh pr merge <n> --squash --delete-branch
```

## Step 6 — Record in codebase

- Full roast → `plans/PR_ROAST_<YYYY_MM_DD>.md` (per-PR verdict table + order + commands).
- Pre-existing flakes/learnings → append `progress/LEARNINGS.md`.
- `plans/GOAP_STATE.md`: update counters in place, set `action_last_completed`
  (exactly once, last key) to the triage action.
