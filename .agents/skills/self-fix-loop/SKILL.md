---
name: self-fix-loop
description: "Automated CI fix cycle and manual test-fix-validate loop. Use after push triggers CI failure for automated repair, or for red-green-refactor cycles on complex changes."
---

# Self-Fix Loop & Iterative Refinement

Automated CI failure remediation AND manual test-fix-validate loops for complex changes.

## When to Use

- After a push that triggers CI failure (automated mode)
- Complex changes spanning multiple files (manual mode)
- Refactoring with test coverage (manual mode)
- New features requiring TDD approach (manual mode)
- When error type is known and fixable programmatically (automated mode)

## Do NOT Use

- When error requires architectural decisions
- When fix needs user input or clarification
- When max iterations reached without resolution
- Single-file simple changes
- Documentation-only updates

## Process

```
┌─────────────────────────────────────────────────┐
│  1. DETECT/RED: Identify failures                │
│  2. CLASSIFY: Categorize error type              │
│  3. GREEN: Apply minimal fix to pass             │
│  4. REFACTOR: Optimize while keeping green       │
│  5. RETRY/VALIDATE: Push fix, re-run checks      │
│  6. REPEAT until PASS or MAX_ITERATIONS          │
└─────────────────────────────────────────────────┘
```

## Loop Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `MAX_ITERATIONS` | 5 (auto) / 10 (manual) | Maximum fix attempts |
| `COOLDOWN` | 30s | Wait between CI check polls |
| `TIMEOUT` | 10m | Max time per iteration |
| `COVERAGE_THRESHOLD` | 80% | Minimum coverage (manual mode) |

## Error Classification & Remediation

| Error Type | Detection Pattern | Fix Strategy |
|------------|-------------------|--------------|
| **shellcheck** | `shellcheck: SC\d+` | Apply suggested fix, quote variables |
| **YAML** | `yaml: line \d+:` | Fix indentation, syntax |
| **lint/clippy** | `error\[E\d+\]` or `clippy::` | Address specific lint warning |
| **security** | `security:`, `vulnerability` | Update dependency, patch CVE |
| **test** | `test failed`, `assertion` | Debug failing test, fix logic |
| **build** | `cannot find`, `unresolved` | Fix imports, add dependencies |
| **Logic error** | Assertion mismatch | Fix implementation logic |
| **Type error** | Compilation failure | Fix types, add conversions |
| **Timeout** | Test exceeds limit | Optimize algorithm |
| **Race condition** | Flaky test | Add synchronization |

## Automated Mode (CI Failure)

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

## Manual Mode (Test-Fix-Validate)

### Phase 1: RED - Identify Failures
```bash
cargo test --all-features --quiet 2>&1 | tee test-output.txt
grep -E "^test .* FAILED" test-output.txt
cargo test --test <test_name> -- --nocapture
```

### Phase 2: GREEN - Apply Fix
1. Analyze failure output
2. Identify root cause (not symptom)
3. Apply minimal fix to make test pass
4. Do NOT refactor yet

### Phase 3: REFACTOR - Optimize
Once tests pass:
1. Identify code smells
2. Apply refactoring patterns
3. Run tests after each change

### Phase 4: VALIDATE
```bash
./scripts/validate.sh
```

## Iteration Tracking

| Iteration | Phase | Action | Result |
|-----------|-------|--------|--------|
| 1 | RED | Run tests | 3 failures |
| 1 | GREEN | Fix imports | Passes |
| 2 | REFACTOR | Extract function | Passes |
| 3 | VALIDATE | Coverage check | 85% covered |

## Exit Criteria

Stop when ALL conditions are met:
- [ ] All tests pass
- [ ] Coverage meets threshold (manual mode)
- [ ] No clippy warnings
- [ ] No performance regression (>10%)
- [ ] LOC gates satisfied

## Escalation Criteria

Stop and escalate when:
- Iteration count exceeds `MAX_ITERATIONS`
- Error type is unclassifiable
- Fix requires architectural change
- Multiple unrelated failures detected

## Safety Constraints

- Never force-push to main/master
- Never skip CI with `--no-verify`
- Always create new commits (never amend existing)
- Don't write tests to pass buggy code
- Don't skip failing tests instead of fixing
- Large refactors need incremental commits
