# The PR Lifecycle — one loop, run in the same order every time

Every PR in this repo takes this path. Skipping a step is how a green PR lands on a red `main`
(#980, 2026-11-02) or how a stale head gets merged (#962/#953). Commands are copy-ready; the
"why" is what the rest of the skill documents.

## 0. Intake — read the state before you touch anything

```bash
git fetch origin --prune
git status --short && git log --oneline -3
gh pr list --state open --json number,title,headRefName,mergeable,mergeStateStatus
gh run list --workflow=ci.yml --branch main --limit 3 \
  --json headSha,conclusion,createdAt
```

Classify before acting: **main red** (fix it first — every PR inherits it), **inherited red**
(the same job is red on main's latest run), **diff red** (the job names a file this branch
changed), **infra** (see §5), **budget** (see §5).

## 1. Verdict — roast before implement or merge

Run `skill://pr-roast-triage`: verify CI truth from job annotations, detect duplicates, check
atomicity, title/body honesty, commitlint scope, preserved rationale, additive-only `deny.toml`,
measured perf claims (bench evidence in the **body**, not a commit message), and no
lockfile/`export.json` noise. No demonstrated impact → close with the roast comment. Record the
verdict in `plans/PR_ROAST_<YYYY_MM_DD>.md`.

## 2. Branch — always off the current `main`

```bash
git switch -c <type>/<scope>-<desc> origin/main   # never commit on main, never a stale base
```

## 3. Local gates — cheapest first, full gate before the push

```bash
# targeted: the gates that own the files you touched (faster than validate.sh, same verdicts)
bash scripts/check-goap-queue-issues.sh      # queue/plans edits
bash scripts/check-docs-links.sh             # .md edits
bash scripts/check-actionlint.sh             # workflow edits
bash scripts/test-workspace-version-edges.sh # fixture touched

# full, for any Rust/workflow/skill change, before the push (not after)
./scripts/validate.sh
npx commitlint --from origin/main --to HEAD --verbose   # grep the WHOLE output for ✖
```

`validate.sh` is the same script the `lint` job runs — if it passes locally, a red `lint` is
infra or an inherited red, not your diff.

## 4. PR — title and body are part of the payload

```bash
gh pr create --title "<type>(<scope>): <imperative>" --body-file /tmp/pr-body.md --base main
```

- `pull_request` runs snapshot the title/body. Set them **before** the push that starts the run.
- Squash subject = `<PR title> (#NNN)`; the budget is `100 - len(" (#NNN)")` (7 for 3 digits).
  `printf '%s (#%s)' "$title" "$N" | wc -c` must be ≤ 100 — branch-side commitlint cannot see it.
- The body carries a `## Test plan` with commands and their measured outcomes, and names any
  red you are *not* fixing plus why.

## 5. CI truth — read the job, then classify

```bash
gh pr checks <n> --json name,bucket     # snapshot, not --watch (TLS can kill the watch)
gh api repos/<o>/<r>/check-runs/<id>/annotations --jq '.[] | {message,path,start_line}'
gh api repos/<o>/<r>/actions/jobs/<id> --jq '.steps[] | {name,conclusion}'
```

Four failure classes, four different responses:

| class | tell | action |
|---|---|---|
| **infra** | annotation says `No space left on device`, `Unable to authenticate`, network/TLS | `gh run rerun <run> --failed`; do not touch code |
| **budget** | `cancelled` at exactly the timeout, a compiler still alive, cache restore logged **success** | raise the budget with the measured numbers in a comment; a re-run only hides it |
| **inherited** | the same job is red on main's latest run | fix main (step 2), rebase, re-run |
| **diff** | the failing step names a file this branch changed | fix the code; re-read the log, not the badge |

A "green PR" is not a green `main`: read `main`'s run for the merge commit after every merge.

## 6. Merge — one PR at a time, from a head that tested the tree

```bash
git fetch origin main && git rebase origin/main && git push --force-with-lease
# wait for CI on the NEW head, then:
gh pr view <n> --json mergeable,mergeStateStatus   # want MERGEABLE + CLEAN
gh pr merge <n> --squash --delete-branch           # NEVER --auto
git fetch origin main && git log --oneline -1 origin/main
```

The ruleset gates too: `code_scanning` / `code_quality` can hold `BLOCKED` ~18 min after every
status context is green. Each merge moves `main`, so the next PR repeats step 6.

## 7. Close-out — the merge is not the end

```bash
# an issue auto-closed by the merge must lose its queue block in the same PR
bash scripts/check-goap-queue-issues.sh
git worktree list && git worktree prune
```

Update `plans/GOAP_STATE.md` (`action_last_completed` exactly once, `queued_actions_count`
re-measured), `plans/ACTIONS.md`, `progress/PROGRESS.md`, `progress/LEARNINGS.md`, then distill
the durable lesson into the owning skill (update an existing one before adding a near-duplicate).
The pre-push queue check passes while the issue is still OPEN — **re-run it after the merge**.

## Anti-patterns (each one measured)

- ❌ pushing to `main`, or merging a head that is behind `main` — a stale head never ran the tree
  that lands
- ❌ `gh pr merge --auto` — it cancels itself the moment `main` moves
- ❌ reading a badge / run-level `success` instead of per-job conclusions (`skipped` is not
  `success`; `cancelled` is not a verdict)
- ❌ re-running a timeout instead of sizing it
- ❌ deleting a `plans/ACTIONS.md` block *after* the merge that closes its issue — that is the red
- ❌ `git add -A` in a dirty worktree, or fixing a red commit by appending instead of rewording
