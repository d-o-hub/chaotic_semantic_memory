---
name: jules-orchestration
description: Orchestrate GitHub issues and automated tasks via the Jules CLI remote interface.
---

# Jules Orchestration

## When to Use
- Orchestrating implementation for GitHub issues (bugs, enhancements).
- Batch applying refactors or patches across multiple modules.
- Automating tasks in a remote environment while keeping a local feedback loop.
- Handling "human-in-the-loop" reviews for AI-generated code.

## Do NOT Use
- For simple local file edits that don't require remote AI assistance.
- When direct `gh` CLI commands are sufficient for non-AI tasks (e.g., labeling, closing issues).
- If the repository has not been initialized with Jules.

## Process

### 1. Triggering Sessions
Dispatch tasks to the Jules agent. Note that the `--parallel` flag triggers multiple agents to work on the **same task description** (useful for alternative solutions). To orchestrate different tasks, use individual commands or a shell loop.

```bash
# Correct pattern for multiple DIFFERENT tasks
jules remote new --session "Task A description"
jules remote new --session "Task B description"

# Using a shell loop for multiple tasks
for task in "Task A" "Task B"; do
  jules remote new --session "$task"
done

# Redundant agents for the SAME task (generating alternatives)
jules remote new --parallel 3 --session "Complex refactor of module X"
```

### 2. Monitoring Progress
Track the status of active sessions.

- **CLI List**: `jules remote list --session` (shows ID, status, and duration).
- **Interactive TUI**: Run `jules` without arguments to open the dashboard. Use this for real-time log monitoring and plan approval.

### 3. Session Management & Limitations
- **No CLI Cancel**: Currently, there is no direct CLI command (e.g., `jules remote cancel`) to stop a running session.
- **Human-in-the-loop**: If a session enters `Awaiting Plan App` or `Paused` status, you **must** enter the Jules TUI (`jules`) to interact with the agent or approve its implementation plan.
- **Redundant Sessions**: If redundant sessions are accidentally triggered (e.g., via `--parallel`), you can either ignore them or use the TUI to monitor which one progresses best. Do NOT pull from multiple sessions for the same task simultaneously.

### 4. Reviewing & Validating
Once a session is `completed`, review the changes before merging.

- **View Diff**: In the Jules TUI, use the side-by-side viewer to inspect changes.
- **Pull Locally**: `jules remote pull --session <id>` to fetch the code into your local branch.

### 4. Error Handling & Refinement
- **Stalled Sessions**: If logs show the agent is stuck or waiting, provide clarification by starting a new session with the same context or using the interactive prompt if available.
- **Merge Conflicts**: If `pull` fails due to conflicts, resolve them locally or task Jules with the resolution: `jules remote new --session "Resolve merge conflicts in the current branch."`
- **Regression Fixes**: If pulled code fails tests, feed the error back: `jules remote new --session "The previous pull for session <id> failed tests with error: [PASTE_ERROR]. Please fix."`

## Parameters
| Parameter | Default | Description |
|-----------|---------|-------------|
| `--parallel` | 1 | Number of concurrent AI sessions to run. |
| `--repo` | (current) | The owner/repo path. Inferred if in a git dir. |
| `--session` | (required) | The prompt/instruction for the agent. |

## Examples

### Implementing an ADR-backed Enhancement
```bash
jules remote new --session "Implement the OTLP exporter as defined in ADR-0072 (plans/adr/0072-otlp-exporter.md)."
```

### Mass Clippy Fixes
```bash
jules remote new --session "Run clippy on the whole project and fix all 'pedantic' warnings related to documentation."
```

## References
- [Jules CLI Reference](https://jules.google/docs/cli/reference)
- @AGENTS.md
- @plans/adr/*.md
