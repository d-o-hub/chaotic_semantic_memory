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

```
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
```bash
git add -A
git commit -m "fix: automated remediation for <error-type>"
git push
```

### 5. Monitor
```bash
gh run watch --exit-status
```

## Escalation Criteria

Stop the loop and escalate when:
- Iteration count exceeds `MAX_ITERATIONS`
- Error type is unclassifiable
- Fix requires architectural change
- Multiple unrelated failures detected

## Example Usage

```
User: "The CI is failing, fix it automatically"

Agent: [Self-Fix Loop activated]
1. Detected: clippy error `unused_variable`
2. Classified: lint warning
3. Fixed: Prefixed with `_` to suppress
4. Retried: Push, CI passes
5. Result: PASS after 1 iteration
```

## Safety Constraints

- Never force-push to main/master
- Never skip CI with `--no-verify`
- Always create new commits (never amend existing)
- Preserve commit message style from repository