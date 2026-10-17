# GitHub Checks (Source of Truth)

Use `gh` CLI:

```bash
gh pr status
gh pr checks --watch
gh run list --branch <branch> --limit 5
gh run view <run-id> --log-failed
```

Rules:
- Do not mark complete if required checks are pending/failing.
- Prefer PR check status over local inference.

## Reading a blocked merge (measured 2026-10-16)

`gh pr checks`'s **default tab-separated rows** carry check names containing spaces
(`Build CLI (linux-x64)`, `Codacy Static Code Analysis`). Parsing that output by whitespace
(`awk '{print $1 $2}'`) silently reads those rows as nameless with no status — which produced a
wrong diagnosis (a superseded run blamed for the block) and a *rerun that itself* put the required
contexts back to `queued`, holding the merge until it finished.

Use these instead:

```bash
# authoritative per-context state for a head (JSON, no parsing hazards)
gh api repos/<owner>/<repo>/commits/<sha>/check-runs \
  --jq '.check_runs[] | "\(.status)\t\(.conclusion)\t\(.name)"'
# the rules that actually gate the merge (branch protection 404 => repository rulesets)
gh api repos/<owner>/<repo>/rulesets --jq '.[] | "\(.id) \(.name) \(.enforcement)"'
gh api repos/<owner>/<repo>/rulesets/<id> --jq '.rules[] | select(.type=="required_status_checks")'
# unresolved review threads, when the ruleset sets required_review_thread_resolution
gh api graphql -f query='{repository(owner:"<o>",name:"<r>"){pullRequest(number:<n>){reviewThreads(first:20){nodes{isResolved path}}}}}'
```

`MERGEABLE/BLOCKED` never says *why*: enumerate the ruleset's requirements instead — every required
context must be `success` (a queued rerun of one is not), the head must be up to date when
`strict_required_status_checks_policy` is on, review threads must be resolved when required, and the
approving-review count must be met. **Re-running a workflow to "clear" a block re-arms the gate**: it
is the right move only when a required context is genuinely absent or stale, never when the reason is
still unknown.

## A `BLOCKED` merge with every required context green (measured 2026-10-17, #909)

The ruleset's non-status rules gate too: `code_scanning` + `code_quality` mean the merge waits for the
CodeQL analyses, so a *pending* `Analyze (rust)` holds the PR at `BLOCKED` after all six required
contexts have gone `success` (it settled ~18 min in). A queued CodeQL job is not a red verdict — check
it before hunting for a code problem.

`gh pr checks --watch` is not a reliable long poll — it exited 1 after ~17 min on
`Post https://api.github.com/graphql: net/http: TLS handshake timeout` while CI itself was fine. Poll
the head's check-runs in a bounded loop (`sleep 60`); a failed call is a retry, never a state.

Version note on the "no `--json`" claim above: gh **2.102.0** does ship `gh pr checks --json fields`
(`bucket`, `state`, `name`, …), and `bucket` categorises `state` into
`pass`/`fail`/`pending`/`skipping`/`cancel` — so `--json name,bucket` is a safe snapshot. The hazard was
always the *default* tab-separated output, not a missing flag; confirm with `gh pr checks --help`.

## After green: land and verify

1. `git fetch origin main` — the branch base must equal current `main`.
2. `mergeStateStatus: CLEAN` — not just `MERGEABLE`.
3. `git diff --stat origin/main...<branch>` — intended files only, no surprise reverts.
4. `gh pr merge <n> --squash --delete-branch` (never `--auto`).
5. Verify the landing: `git ls-remote origin 'refs/heads/<branch>'` → 0 refs;
   `git log --oneline -1 origin/main` → the squash subject.
