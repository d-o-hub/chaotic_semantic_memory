---
name: iterative-refinement
description: "Test-fix-validate loops for complex changes: run tests, identify failures, fix, validate, repeat until green. Use for red-green-refactor cycles."
---

# Iterative Refinement

Test-fix-validate loops for complex changes requiring multiple iterations.

## When to Use

- Complex changes spanning multiple files
- Refactoring with test coverage
- New features requiring TDD approach
- Bug fixes needing verification

## Do NOT Use

- Single-file simple changes
- Documentation-only updates
- Changes without existing test coverage

## Process

```
┌─────────────────────────────────────────────────┐
│  RED:    Run tests, identify failures           │
│  GREEN:  Apply minimal fix to pass              │
│  REFACTOR: Optimize while keeping green         │
│  REPEAT until coverage passes and clean         │
└─────────────────────────────────────────────────┘
```

## Loop Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `MAX_ITERATIONS` | 10 | Maximum refinement cycles |
| `COVERAGE_THRESHOLD` | 80% | Minimum coverage to accept |
| `BENCH_BASELINE` | main | Branch to compare benchmarks |

## Step-by-Step Execution

### Phase 1: RED - Identify Failures

```bash
# Run full test suite
cargo test --all-features --quiet 2>&1 | tee test-output.txt

# Parse failures
grep -E "^test .* FAILED" test-output.txt

# For specific test debugging
cargo test --test <test_name> -- --nocapture
```

### Phase 2: GREEN - Apply Fix

1. Analyze failure output
2. Identify root cause (not symptom)
3. Apply minimal fix to make test pass
4. Do NOT refactor yet

```bash
# Quick validation
cargo check --message-format=short
cargo test --all-features --quiet
```

### Phase 3: REFACTOR - Optimize

Once tests pass:
1. Identify code smells
2. Apply refactoring patterns
3. Run tests after each change
4. Verify no regression

### Phase 4: VALIDATE

```bash
# Full gate sequence
./scripts/validate.sh

# Coverage check (if configured)
cargo tarpaulin --all-features --out Stdout

# Benchmark comparison
cargo bench --bench benchmark -- --baseline main
```

## Failure Categories

| Category | Pattern | Approach |
|----------|---------|----------|
| **Logic error** | Assertion mismatch | Fix implementation logic |
| **Type error** | Compilation failure | Fix types, add conversions |
| **Timeout** | Test exceeds limit | Optimize algorithm |
| **Race condition** | Flaky test | Add synchronization |
| **Environment** | Missing dependency | Fix setup, add config |

## Iteration Tracking

Track each iteration:

| Iteration | Phase | Action | Result |
|-----------|-------|--------|--------|
| 1 | RED | Run tests | 3 failures |
| 1 | GREEN | Fix imports | Passes |
| 2 | REFACTOR | Extract function | Passes |
| 3 | VALIDATE | Coverage check | 85% covered |

## Exit Criteria

Stop when ALL conditions are met:
- [ ] All tests pass
- [ ] Coverage meets threshold
- [ ] No clippy warnings
- [ ] No performance regression (>10%)
- [ ] LOC gates satisfied

## Example Workflow

```
User: "Refactor the reservoir module"

Agent: [Iterative Refinement activated]
Iteration 1:
  RED: cargo test → 2 failures in reservoir
  GREEN: Fix borrow checker issue
  Result: Tests pass

Iteration 2:
  REFACTOR: Extract metrics to separate file
  RED: cargo test → 0 failures
  Result: Still green

Iteration 3:
  VALIDATE: ./scripts/validate.sh
  Result: All gates pass, 92% coverage

Final: Refactoring complete after 3 iterations
```

## Anti-Patterns to Avoid

- Writing tests to pass buggy code
- Skipping failing tests instead of fixing
- Large refactors without incremental commits
- Ignoring performance regressions