---
name: task-decomposition
description: Break complex tasks into atomic, dependency-ordered goals. Use for multi-step tasks with unclear boundaries or when planning agent team assignments.
---

# Task Decomposition

## When to Use

- Complex multi-step tasks (3+ distinct actions)
- Tasks requiring multiple files or modules
- Work that could be parallelized across agents
- Unclear implementation path
- Before spawning an agent team

## Process

```
1. Define Outcome
   - What does "done" look like?
   - Acceptance criteria
   - Verification method

2. Identify Subtasks
   - List all discrete work items
   - Mark blockers/dependencies
   - Estimate relative effort (S/M/L)

3. Order by Dependency
   - Draw dependency graph
   - Identify parallelizable work
   - Sequence blocking tasks first

4. Validate Completeness
   - Does completing all subtasks achieve the outcome?
   - Are there hidden dependencies?
   - Is effort estimate realistic?
```

## Work Breakdown Structure

| Size | Lines Changed | Files | Time Est. |
|------|---------------|-------|-----------|
| S    | < 50          | 1-2   | < 30 min  |
| M    | 50-200        | 2-4   | 30-90 min |
| L    | > 200         | 4+    | > 90 min  |

## Dependency Patterns

### Sequential (Blocking Chain)
```
Task A (no deps) → Task B (depends on A) → Task C (depends on B)
```
Use `addBlockedBy` to enforce order.

### Parallel (Independent)
```
Task A ─┐
Task B ─┼─→ Integration Task (depends on A, B, C)
Task C ─┘
```
Tasks A, B, C can run concurrently.

### Mixed
```
Task A → Task B ─┐
                 ├─→ Task E
Task C → Task D ─┘
```

## Example Decomposition

**Task**: "Add export/import feature for concepts"

| ID | Subtask | Depends On | Size | Owner |
|----|---------|------------|------|-------|
| 1 | Design JSON schema for export | - | S | - |
| 2 | Implement exporter in persistence.rs | 1 | M | - |
| 3 | Implement importer in persistence.rs | 1 | M | - |
| 4 | Add CLI commands (export/import) | 2, 3 | M | - |
| 5 | Write integration tests | 4 | M | - |
| 6 | Update documentation | 4 | S | - |

## Task Blocking with TaskUpdate

```json
TaskUpdate(taskId: "5", addBlockedBy: ["4"])  // Tests wait for CLI
TaskUpdate(taskId: "4", addBlockedBy: ["2", "3"])  // CLI waits for impl
```

## Anti-Patterns

| Avoid | Instead |
|-------|---------|
| Vague tasks ("fix stuff") | Specific outcome ("reduce bench time by 20%") |
| Missing dependencies | Explicit `addBlockedBy` calls |
| Giant tasks | Split into S/M subtasks |
| Circular dependencies | Re-plan the breakdown |

## Verification Checklist

- [ ] Each task has clear acceptance criteria
- [ ] Dependencies are explicit (not assumed)
- [ ] No task depends on itself (directly or transitively)
- [ ] Parallel tasks identified for team efficiency
- [ ] Integration/verification task included