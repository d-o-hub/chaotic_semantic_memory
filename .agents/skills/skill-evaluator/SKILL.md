---
name: skill-evaluator
description: "Evaluate skill performance with benchmarks and metrics. Use when validating skill effectiveness or measuring improvement from skill usage."
---

# Skill Evaluator

Measure and report on skill effectiveness.

## When to Use

- Validating a newly created skill
- Measuring improvement from skill adoption
- Auditing existing skills for effectiveness
- Comparing skill variants

## Do NOT Use

- For one-off task execution
- When no measurable outcomes exist

## Process

```
┌─────────────────────────────────────────────────┐
│  1. DEFINE: Set benchmark and success criteria  │
│  2. RUN: Execute skill on test cases             │
│  3. MEASURE: Collect performance metrics         │
│  4. REPORT: Summarize outcomes and trends        │
└─────────────────────────────────────────────────┘
```

## Metrics Framework

| Metric | Description | Target |
|--------|-------------|--------|
| **Success Rate** | Tasks completed / attempted | >90% |
| **Time Saved** | Manual time - skill time | >50% |
| **Error Reduction** | Errors before - after | >70% |
| **Consistency** | Same input = same output | 100% |

## Evaluation Types

| Type | Method | Use Case |
|------|--------|----------|
| **Functional** | Does it work? | Correctness validation |
| **Performance** | How fast? | Efficiency measurement |
| **Usability** | How easy? | User experience |
| **Reliability** | How often fails? | Stability assessment |

## Step-by-Step Evaluation

### 1. Define Benchmark

Create test cases that represent typical usage:

```markdown
# Benchmark Cases

| Case | Input | Expected Output |
|------|-------|-----------------|
| Basic | Simple trigger | Minimal viable output |
| Complex | Multi-step request | Complete workflow |
| Edge | Unusual input | Graceful handling |
| Stress | Large input | Performance under load |
```

### 2. Run Skill

Execute skill on each test case:

```bash
# Record timing
START=$(date +%s)
# Execute skill...
END=$(date +%s)
DURATION=$((END - START))
```

### 3. Measure Outcomes

Collect metrics for each run:

| Run | Case | Time | Success | Notes |
|-----|------|------|---------|-------|
| 1 | Basic | 2s | Yes | Clean execution |
| 2 | Complex | 15s | Yes | Minor retry |
| 3 | Edge | 8s | Partial | Needed intervention |

### 4. Generate Report

```markdown
# Skill Evaluation Report

## Summary
- Skill: <name>
- Date: <date>
- Evaluator: <agent/user>

## Results
| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Success Rate | 85% | 90% | Near |
| Avg Time | 8.3s | 10s | Pass |
| Consistency | 100% | 100% | Pass |

## Findings
- Strong: Fast execution, consistent output
- Weak: Edge case handling
- Recommendation: Add edge case handling
```

## Evaluation Templates

### Functional Test

```markdown
## Functional Evaluation

### Test Cases
1. [ ] Basic trigger works
2. [ ] Multi-step process completes
3. [ ] Error handling works
4. [ ] Output format correct

### Results
Pass: X/4
```

### Performance Test

```markdown
## Performance Evaluation

### Benchmarks
| Operation | Time | Baseline | Status |
|-----------|------|----------|--------|
| Init | 0.5s | 1s | Pass |
| Execute | 2s | 5s | Pass |
| Cleanup | 0.1s | 0.5s | Pass |
```

### Usability Test

```markdown
## Usability Evaluation

### Questions
- [ ] Documentation clear?
- [ ] Steps easy to follow?
- [ ] Error messages helpful?
- [ ] Output format useful?

### Score: X/4
```

## Comparative Evaluation

When comparing skill variants:

| Variant | Success | Time | Quality |
|---------|---------|------|---------|
| A (current) | 90% | 5s | High |
| B (proposed) | 95% | 3s | High |
| Winner | B | B | Tie |

## Automation Integration

Skills can self-evaluate:

```markdown
# Self-Evaluation Checklist
- [ ] All process steps executed
- [ ] Output matches expected format
- [ ] No errors in execution
- [ ] Time within acceptable range
```

## Reporting Frequency

| Skill Type | Frequency | Method |
|------------|-----------|--------|
| Critical | Every use | Auto-log |
| Standard | Weekly | Manual audit |
| Experimental | Per project | Full report |

## Example Evaluation

```
User: "Evaluate the rust-development skill"

Agent: [Skill Evaluator activated]
1. DEFINE: 5 test cases (basic, complex, edge, refactor, docs)
2. RUN: Execute each with timing
3. MEASURE:
   - Success: 4/5 (80%, edge case failed)
   - Avg time: 45s per task
   - Consistency: 100%
4. REPORT:
   - Skill effective for standard work
   - Needs edge case handling improvement
   - Recommendation: Add error recovery steps
```

## Improvement Cycle

1. Evaluate → 2. Identify gaps → 3. Update skill → 4. Re-evaluate

Track improvements over time:

| Version | Date | Success | Notes |
|---------|------|---------|-------|
| 1.0 | 2026-01 | 75% | Initial |
| 1.1 | 2026-02 | 85% | Added edge cases |
| 1.2 | 2026-03 | 92% | Improved docs |