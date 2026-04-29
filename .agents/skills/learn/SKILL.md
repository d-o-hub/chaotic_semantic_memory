---
name: learn
description: Capture non-obvious session learnings to project memory. Use at end of session or after solving hard problems to persist insights for future sessions.
---

# Learn Skill

## When to Use

- End of coding session (before shutdown)
- After solving a non-trivial debugging problem
- When discovering patterns that aren't documented elsewhere
- After hitting unexpected behavior that took time to diagnose

## Process

1. **Identify Insight**: What did you learn that wasn't obvious?
2. **Verify Novelty**: Check MEMORY.md and existing docs - is this already captured?
3. **Formulate Pattern**: Write as actionable guidance, not just "I learned X"
4. **Save to Memory**: Add to appropriate section in MEMORY.md

## Pattern Format

Match MEMORY.md style:

```markdown
## Section Name

- **Topic**: Concise explanation with actionable guidance
- **Key insight**: Non-obvious behavior or gotcha
- **Correct pattern**: Code or command example
- **Wrong pattern**: What to avoid (optional)
```

## Example Insights Worth Capturing

| Category | Example |
|----------|---------|
| Tool quirks | `find with head -n 1 is unreliable - use explicit filters` |
| API gotchas | `libsql Statement requires reset() between executions` |
| Performance | `Batch size 512 amortizes Rayon overhead for 1000+ candidates` |
| Debugging | `Check which binary `which csm` resolves to - PATH shadowing` |
| Process | `Pre-commit hook must include clippy - CI catches what local misses` |

## What NOT to Capture

- Session-specific context (e.g., "today I fixed bug X")
- Already-documented patterns
- Trivial observations
- Information that will quickly become stale

## Integration with MEMORY.md

MEMORY.md structure (keep under 200 lines):
- Architecture section for core concepts
- Key Files section for important modules
- Feature Flags for configuration
- Coding Standards for conventions
- CI/Script Lessons for hard-won process insights
- Performance Optimization Notes for benchmarks

## Workflow

```
End Session
    |
    v
Reflect: What was non-obvious?
    |
    v
Check MEMORY.md - already there?
    |         |
   Yes       No
    |         |
    v         v
  Skip    Add concise entry
              |
              v
         Keep under 200 lines total
```