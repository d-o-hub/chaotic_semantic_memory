---
name: self-fix-loop
description: "Automated CI fix cycle: detect failure, classify error, apply fix, retry. Use after push triggers CI failure for automated repair iteration."
---

# Self-Fix Loop

Automated CI failure remediation with iterative repair attempts.

## When to Use

- After a push that triggers CI failure
- When error type is known and fixable programmatically
- For repeatable, deterministic error patterns

## Do NOT Use

- When error requires architectural decisions
- When fix needs user input or clarification
- When max iterations reached without resolution

## Process

```text
┌─────────────────────────────────────────────────┐
│  1. DETECT: Fetch CI logs, identify failure      │
│  2. CLASSIFY: Categorize error type             │
│  3. FIX: Apply appropriate remediation         │
│  4. RETRY: Push fix, re-run CI                  │
│  5. REPEAT until PASS or MAX_ITERATIONS (5)     │
└─────────────────────────────────────────────────┘
```

## Error Classification & Remediation

| Error Type | Detection Pattern | Fix Strategy |
|------------|-------------------|--------------|
| **shellcheck** | `shellcheck: SC\d+` | Apply suggested fix, quote variables |
| **YAML** | `yaml: line \d+:` | Fix indentation, syntax |
| **lint/clippy** | `error\[E\d+\]` or `clippy::` | Address specific lint warning |
| **security** | `security:`, `vulnerability` | Update dependency, patch CVE |
| **test** | `test failed`, `assertion` | Debug failing test, fix logic |
| **build** | `cannot find`, `unresolved` | Fix imports, add dependencies |

## Loop Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `MAX_ITERATIONS` | 5 | Maximum fix attempts before escalation |
| `COOLDOWN` | 30s | Wait between CI check polls |
| `TIMEOUT` | 10m | Max time per iteration |

## Step-by-Step Execution

### 1. Detect Failure
```bash
gh run list --limit 1 --json conclusion,status,databaseId
gh run view <run-id> --log-failed
```

### 2. Classify Error
Parse logs for known patterns. Map to fix strategy.

### 3. Apply Fix
Execute appropriate remediation based on classification:
- **shellcheck**: `shellcheck -f diff script.sh | git apply`
- **clippy**: Address specific warnings with code changes
- **test**: Analyze failure, modify test or code

### 4. Retry

Stage only the intended tracked paths before running
`scripts/self-fix-loop.sh`; it refuses detached HEAD, `main`, branch mismatch,
unstaged edits, and untracked files. It commits the staged set only—never
`git add -A`—and refuses to rebase automatically after a rejected push.

Before each push, type the exact `origin/<branch>@<sha>` target shown by the
script. Non-interactive use requires setting
`SELF_FIX_LOOP_CONFIRM_PUSH=origin/<branch>@<sha>` for the commit being pushed.
Review the SHA and target before confirming; a token for another branch or SHA
does not authorize the push.

### 5. Monitor
```bash
gh run watch --exit-status
```

Read the verdict, never the wrapper. Two traps that have each produced a false green in
this repo:

- A backgrounded command run as `cmd; echo "RC=$?"` reports the *`echo`*'s exit code in
  the completion notification, so the task says "exit code 0" while `RC=1` is in the log.
  Capture the code into the log file (`cmd; echo "RC=$?" >> log`) or run bare.
- Anything through a pipe reports the last process: `./scripts/validate.sh | tail -40`
  yields `tail`'s 0. Same for `&&`-chains used as "run then show".

### 6. A rejected push: classify before retrying

`git fetch` and read `origin/<branch>` history — a bot/owner merge often lands on the PR
branch mid-loop. If the remote addition touches **none** of your paths, replay your commit
onto it (`git rebase origin/<branch>`, then a fast-forward push) so the author's merge
commit survives; if it overlaps, stop and re-resolve deliberately. Never `--force`, and
never treat the refusal as a transient network error.

## Escalation Criteria

Stop the loop and escalate when:
- Iteration count exceeds `MAX_ITERATIONS`
- Error type is unclassifiable
- Fix requires architectural change
- Multiple unrelated failures detected
- The fix is a CI gate that reads a *generated* counter (`validate_llm_context.py`
  compares `modules_under_loc_limit` against live `src/**/*.rs`, and `tests_count` against
  `total_tests`/`tests_passing`): the number must be re-measured from the tree, not typed.
  Count test registrations from harness output (`N passed; M filtered out`) — `grep
  '#\[test\]'` is blind to `#[tokio::test]` and `#[test_case]`.

## Example Usage

```text
User: "The CI is failing, fix it automatically"

Agent: [Self-Fix Loop activated]
1. Detected: clippy error `unused_variable`
2. Classified: lint warning
3. Fixed: addressed the unused variable at its source
4. Retried: Push, CI passes after exact-target confirmation
5. Result: PASS after 1 iteration
```

## Safety Constraints

- Refuse `main` and detached HEAD before any commit or remote operation.
- Require a clean unstaged/untracked worktree; commit only operator-staged files.
- Require an exact remote/branch/SHA confirmation before each push.
- A remote/branch/SHA mismatch is a hard stop: do not push, bypass the refusal, or skip hooks/CI. Reinspect the exact target and require matching confirmation before any later push attempt.
- Never use `git add -A`, force-push, or auto-rebase.
- Never skip CI with `--no-verify`.
- Always create new commits (never amend existing).
- Preserve commit message style from repository
