# CLAUDE.md - Claude Code Specific Instructions

This file contains instructions specific to Claude Code CLI. For general project instructions, see [AGENTS.md](AGENTS.md).

## Claude Code Features

- **Auto-memory**: `~/.claude/projects/-home-do-git-chaotic-semantic-memory/memory/`
- **Agent Teams**: See https://code.claude.com/docs/en/agent-teams
- **Swarm Mode**: Use `TeamCreate` to spawn coordinated agents

## Session Continuity

When continuing from a compacted session, clean up stale resources:

```bash
rm -rf ~/.claude/teams/<team-name> ~/.claude/tasks/<team-name>
```

Or use `/exit` to terminate all in-process agents.

## In-Process Agents

Agents with `backendType: "in-process"` share the main Claude process. They:
- Cannot be killed by shutdown requests
- Persist until the session ends
- Should be cleaned up via `TeamDelete` or session exit