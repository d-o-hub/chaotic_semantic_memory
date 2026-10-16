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

`gh pr checks` has **no `--json` flag**, and its tab-separated rows carry check names containing
spaces (`Build CLI (linux-x64)`, `Codacy Static Code Analysis`). Parsing that output by whitespace
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
