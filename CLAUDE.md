# CLAUDE.md - Claude Code Specific Instructions

This file contains instructions specific to Claude Code CLI. For general project instructions, see [AGENTS.md](AGENTS.md).

---

## Agent Teams (TeamCreate)

Use `TeamCreate` to spawn coordinated agents for complex multi-file tasks.

### Team Workflow

```
TeamCreate → TaskCreate (for each subtask) → Agent (spawn teammates) → TaskUpdate (assign) → Monitor → Shutdown → TeamDelete
```

### Example: Parallel Documentation Sync

```yaml
# 1. Create team
TeamCreate(team_name: "phase55-doc-sync", description: "Documentation sync")

# 2. Create tasks
TaskCreate(subject: "Update AGENTS.md")
TaskCreate(subject: "Update README.md")

# 3. Spawn agents with team_name
Agent(name: "doc-writer-1", team_name: "phase55-doc-sync", prompt: "Update AGENTS.md...")
Agent(name: "doc-writer-2", team_name: "phase55-doc-sync", prompt: "Update README.md...")

# 4. Assign tasks
TaskUpdate(taskId: "1", owner: "doc-writer-1")
TaskUpdate(taskId: "2", owner: "doc-writer-2")

# 5. Monitor via idle notifications (automatic)

# 6. Shutdown when complete
SendMessage(to: "doc-writer-1", message: {type: "shutdown_request", reason: "Complete"})

# 7. Clean up
TeamDelete()
```

### Agent Types (subagent_type)

| Type | Use Case | Tools |
|------|----------|-------|
| `general-purpose` | Full implementation | All tools |
| `Explore` | Codebase exploration | Glob, Grep, Read (no Edit) |
| `Plan` | Architecture planning | All tools (no Edit/Write) |

### Task Dependency Management

```yaml
# Blocked task waits for dependency
TaskUpdate(taskId: "2", addBlockedBy: ["1"])  # Task 2 waits for Task 1

# Check blocked status
TaskList()  # Shows blockedBy for each task

# After Task 1 completes, Task 2 becomes unblocked
```

---

## Specialist Skills

Skills are loaded on-demand via `/skill-name` or auto-triggered by description match.

### Core Skills

| Skill | Trigger | Purpose |
|-------|---------|---------|
| `rust-development` | Rust code changes | Implement/refactor modules |
| `testing-validation` | Validation gates | Compile/test/lint/LOC |
| `goap-planning` | Planning tasks | Build action plans from state |
| `adr-creation` | Architecture changes | Write ADR documents |
| `github-ci-guardrails` | Pre-merge checks | Validate CI via gh CLI |
| `git-workflow` | Git operations | Commit conventions, CI/CD |
| `release-management` | Publishing | GitHub releases, crates.io |
| `benchmarking-perf` | Performance | Criterion benchmarks |

### Swarm Skills (Parallel Groups)

| Skill | Focus Area |
|-------|------------|
| `swarm-testing-quality` | Property testing, fuzzing |
| `swarm-performance` | SIMD, pooling, caching |
| `swarm-observability` | Tracing, metrics |
| `swarm-advanced-features` | Export/import, migrations |

### Skill Invocation

```bash
# Manual invoke
/skill-name

# Auto-trigger (skill description matches task)
# Skills are in .claude/skills/<skill>/SKILL.md
```

---

## Hooks System

Hooks are deterministic callbacks in `.claude/settings.json`. Where AGENTS.md is advisory (~70% followed), hooks are mandatory.

### Hook Types

| Hook | When | Use Case |
|------|------|----------|
| `PreToolUse` | Before tool call | Route risky ops to Opus |
| `PostToolUse` | After tool call | Auto-format after edit |
| `Stop` | Before "done" | Verify work complete |

### Example: PostToolUse Auto-Format

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit",
        "hooks": ["cargo fmt --quiet"]
      }
    ]
  }
}
```

---

## Plan Mode

Use `EnterPlanMode` for tasks requiring 3+ steps or architectural decisions.

### Plan Mode Workflow

```
EnterPlanMode → Explore → Write Plan → ExitPlanMode (requests approval) → Implement
```

### When to Use

- Adding new features (architectural decisions)
- Refactoring multiple files
- Making trade-off decisions
- User preference matters

### When NOT to Use

- Simple fixes (typos, obvious bugs)
- Single-file edits with clear requirements
- Research/exploration tasks

---

## Auto-Memory

Memory persists across conversations at:
```
~/.claude/projects/-home-do-git-chaotic-semantic-memory/memory/
```

### Memory Files

- `MEMORY.md` — Always loaded (keep under 200 lines)
- Topic files (e.g., `debugging.md`, `patterns.md`) — Linked from MEMORY.md

### Memory Rules

- Save stable patterns confirmed across multiple interactions
- Don't save session-specific context
- Update/remove outdated memories
- Never duplicate CLAUDE.md or AGENTS.md content

---

## Session Management

### Cleanup Stale Resources

```bash
rm -rf ~/.claude/teams/<team-name> ~/.claude/tasks/<team-name>
```

Or use `/exit` to terminate all in-process agents.

### In-Process Agents

Agents with `backendType: "in-process"` share the main Claude process:
- Cannot be killed by shutdown requests
- Persist until session ends
- Clean up via `TeamDelete` or session exit

### Multi-Session (Boris Method)

- Each session uses its own Git worktree
- `/compact` at 50% context, `/clear` when switching tasks
- Plan → Execute → Verify loop

---

## Message Protocol

### Shutdown Request/Response

```json
// Request
SendMessage(to: "agent-name", message: {type: "shutdown_request", reason: "Complete"})

// Response (agent must send)
SendMessage(to: "team-lead", message: {type: "shutdown_response", request_id: "xxx", approve: true})
```

### Plan Approval

```json
// Request (from ExitPlanMode)
SendMessage(to: "team-lead", message: {type: "plan_approval_request", request_id: "xxx"})

// Response
SendMessage(to: "planner", message: {type: "plan_approval_response", request_id: "xxx", approve: true})
```

---

## Context Efficiency

- Keep CLAUDE.md under 200 lines
- Each line must earn its place
- Reference files via `@path/to/file` syntax
- Use skills for on-demand loading (not every session)