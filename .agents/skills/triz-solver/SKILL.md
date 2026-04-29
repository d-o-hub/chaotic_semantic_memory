---
name: triz-solver
description: "Apply TRIZ inventive principles to solve problems that seem to have no good solution. Use after triz-analysis has identified contradictions."
---

# TRIZ Solver

## When to Use
- After completing TRIZ analysis (triz-analysis skill)
- When standard solutions don't work
- When tradeoffs seem unavoidable
- For creative problem-solving in architecture

## Process

### 1. Review Contradictions
Load contradictions identified in `plans/triz/` analysis.

### 2. Select Primary Principle
From `references/principles.md` (via triz-analysis), choose the most applicable principle.

### 3. Apply the Principle
Follow the principle's specific technique:

#### Segmentation (#1)
- Split the problem into independent parts
- Apply different solutions to each part
- **Example**: Different search strategies for different query sizes

#### Taking out (#2)
- Extract the problematic element
- Keep only essential functionality
- **Example**: Extract rate limiting to middleware

#### Inversion (#12)
- Reverse the control flow
- **Example**: IoC container, dependency injection

#### Feedback (#23)
- Add feedback loops
- **Example**: Auto-tuning based on metrics

#### Composite Materials (#40)
- Combine multiple approaches
- **Example**: Hybrid keyword + semantic search

### 4. Validate Solution
Check against IFR:
- Does it move toward the ideal?
- Does it resolve the contradiction?
- Are there new problems introduced?

### 5. Document Resolution
Update `plans/triz/` with:
- Chosen principle(s)
- Implementation approach
- Tradeoffs accepted
- Validation results

## Solution Patterns

### Pattern: Strategy Segmentation
When: Performance varies by input characteristics
Apply: Principle #1 (Segmentation) + #3 (Local quality)
Solution: Dispatch table routing to specialized handlers

### Pattern: Inverted Control
When: Flexibility creates complexity
Apply: Principle #12 (Inversion)
Solution: DI container with declarative wiring

### Pattern: Composite Approach
When: Single approach has fatal flaw
Apply: Principle #40 (Composite)
Solution: Combine approaches, use best of each

### Pattern: Preliminary Action
When: Runtime costs are high
Apply: Principle #10 (Preliminary action)
Solution: Pre-compute, cache, or compile at build time

### Pattern: Feedback Loop
When: Static solution is suboptimal
Apply: Principle #23 (Feedback)
Solution: Adaptive algorithm with runtime adjustment

## Anti-Patterns to Avoid

1. **Applying principle superficially**: Don't just rename, restructure
2. **Ignoring IFR**: Keep the ideal as the goal
3. **Accepting tradeoffs too early**: Principles resolve contradictions
4. **Single principle fixation**: Combine 2-3 principles for complex problems

## Example Session

```text
Problem: Retrieval pipeline has slow cold starts but caching adds complexity.

IFR: Pipeline warms itself without explicit cache management.

Contradiction: Speed vs Simplicity

Principles Applied:
- #25 Self-service: Cache populates on first access
- #23 Feedback: Track access patterns
- #15 Dynamicity: Adjust caching based on patterns

Solution: Lazy population with adaptive eviction based on access frequency.
```

## Integration with Other Skills

| Skill | When to Hand Off |
|-------|-----------------|
| adr-creation | Document architectural decision |
| goap-planning | Plan implementation actions |
| testing-validation | Verify solution works |
| rust-development | Implement the solution |