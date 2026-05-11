# CLAUDE.md - Claude Code CLI Instructions

**General project workflow**: See [AGENTS.md](AGENTS.md) for coding phases, validation gates, and session checklist.

---

## Agent Teams (TeamCreate)

Spawn coordinated agents for complex multi-file tasks.

```
TeamCreate → TaskCreate → Agent (spawn) → TaskUpdate (assign) → Monitor → Shutdown → TeamDelete
```

| subagent_type | Tools | Use Case |
|---------------|-------|----------|
| `general-purpose` | All | Full implementation |
| `Explore` | Glob, Grep, Read | Codebase exploration |
| `Plan` | All (no Edit/Write) | Architecture planning |

**Task blocking**: `TaskUpdate(taskId: "2", addBlockedBy: ["1"])`

---

<!-- SKILLS_TABLE_START -->
## Specialist Skills

Loaded on-demand via `/skill-name` or auto-triggered by description.

| Core Skills | Purpose |
|-------------|---------|
| `adr-creation` | Write or update ADRs for architecture-impacting ch |
| `benchmarking-perf` | Run and analyze criterion benchmarks for performan |
| `debugging-reservoir` | Debug and tune the echo state network reservoir |
| `dist-channel-selection` | Artifact-aware decision logic for selecting the co |
| `drawio` | Create high-level architecture diagrams using draw |
| `git-workflow` | Git commit conventions, validation gates, and CI/C |

| Swarm Skills | Focus |
|--------------|-------|
| `analysis-swarm` | Multi-persona code analysis orchestrator using RYA |
| `swarm-advanced-features` | Export/import, versioning, migrations, and backup/ |
| `swarm-observability` | Tracing, metrics, derive macros, and error context |

<!-- SKILLS_TABLE_END -->
## Hooks System

Mandatory callbacks in `.claude/settings.json` (AGENTS.md is advisory ~70%).

| Hook | When | Example |
|------|------|---------|
| `PreToolUse` | Before tool | Route risky ops to Opus |
| `PostToolUse` | After tool | `cargo fmt --quiet` on Edit |
| `Stop` | Before done | Verify work complete |

---

## Plan Mode

Use `EnterPlanMode` for 3+ steps or architectural decisions.

**Workflow**: `EnterPlanMode → Explore → Write Plan → ExitPlanMode → Implement`

**Avoid for**: Simple fixes, single-file edits, research tasks.

---

## Auto-Memory

Persists at: `~/.claude/projects/-home-do-git-chaotic-semantic-memory/memory/`

- `MEMORY.md` — Always loaded (<200 lines)
- Topic files — Linked from MEMORY.md

**Rules**: Save stable patterns only; no session context; never duplicate AGENTS.md.

---

## Session Management

- **Cleanup**: `rm -rf ~/.claude/teams/<team>` or `/exit`
- **In-process agents**: Persist until session ends (can't be killed by shutdown)
- **Multi-session**: Git worktree per session; `/compact` at 50% context

---

## Message Protocol

```json
// Shutdown
SendMessage(to: "agent", message: {type: "shutdown_request", reason: "..."})
SendMessage(to: "lead", message: {type: "shutdown_response", request_id: "x", approve: true})

// Plan approval
SendMessage(to: "lead", message: {type: "plan_approval_response", request_id: "x", approve: true})
```

---

## Context Efficiency

- Keep CLAUDE.md under 200 lines
- Reference files via `@path/to/file`
- Skills for on-demand loading