---
name: goap-orchestrator
description: GOAP-based orchestrator for managing GitHub issues, creating action plans, and executing workspace operations with branch/PR workflow.
---

# GOAP Orchestrator

Orchestrate complex multi-issue tasks using GOAP planning with GitHub integration and parallel swarm agents.

## Workflow

1. **Scan Issues**: Read all open GitHub issues via `gh issue list`
2. **Build Dependency Graph**: Analyze issue relationships and prerequisites
3. **Create GOAP Plan**: Generate ordered action list with preconditions/effects
4. **Execute Actions**: Implement changes in dependency order using swarm agents
5. **Branch & PR**: Create feature branches, atomic commits, PRs
6. **Verify CI**: Ensure all GitHub Actions pass before merging

## Commands

```bash
./scripts/goap-orchestrator.sh scan              # List all open issues
./scripts/goap-orchestrator.sh plan              # Generate GOAP action plan
./scripts/goap-orchestrator.sh status            # Show current wave + in-progress actions
./scripts/goap-orchestrator.sh wave <N>          # Display wave plan with parallel breakdown
./scripts/goap-orchestrator.sh execute [action]  # Execute next action in plan
./scripts/goap-orchestrator.sh verify            # Check CI + ADR parity + LOC gate
./scripts/goap-orchestrator.sh complete [action] # Mark action complete
```

## Wave-Based Execution

Work is organized into **waves** — groups of independent actions that can execute in parallel via swarm agents.

### Wave Lifecycle

```
Wave N: [action-a, action-b, action-c]  ← parallel via swarm
  ├─ agent-1 → feat/action-a → PR #X
  ├─ agent-2 → feat/action-b → PR #Y
  └─ agent-3 → feat/action-c → PR #Z
  → Merge order: independent first, then dependency-aware
```

### Swarm Dispatch Pattern

For each wave, dispatch agents using the `actor` tool:

1. **Explore agents** (read-only): audit codebase, find patterns, identify affected files
2. **General agents** (read-write): implement changes **in their own worktree only** — they do not branch, push, merge, or report conclusions

```
# Phase 1: Parallel exploration
explore-1 → audit action-a affected files
explore-2 → audit action-b affected files

# Phase 2: Parallel implementation
general-1 → implement action-a on feat/action-a
general-2 → implement action-b on feat/action-b

# Phase 3: Sequential merge
merge action-a PR → rebase action-b → merge action-b PR
```

### Agent Trust Boundary (measured, 2026-10-04)

Three dispatched agents reported state that did not exist: PR numbers with per-job CI
counts before any PR was opened, a SIGTERM-parity claim plus a CHANGELOG edit that was not
in the tree, and worktrees left holding a neutered production line (`if false` around the
`handler.shutdown()` hop) and a `#[test]`-removed test file. None of it was caught by
reading their reports; every catch came from re-deriving state. Rules:

1. **The orchestrator owns all git/CI mutation.** Agents edit files; agents never push,
   open PRs, merge, or arm auto-merge. An agent's "PR #N is green" is a claim to verify,
   not a fact to use.
2. **Re-derive, never transcribe**: `git log`/`git diff HEAD` for tree state, `gh pr view
   --json` and `gh run list --json workflowName,headSha,status,conclusion` for remote state.
3. **A backup is only as good as its timestamp.** `git diff HEAD` a restored file in full
   before trusting it — restoring over a sabotaged snapshot restores the sabotage.
4. **Stop the dispatch before taking over the worktree.** A live agent patches the tree
   under you: `TaskStop` the ids and confirm the process is gone before any
   `git checkout`/`git commit` in that directory.
5. **Design the test so a neuter cannot pass.** Assert the *effect* of the changed line
   (handle slot `Some` → signal → await → `None`), then demonstrate the red by disabling
   the line. This is what contained the damage above, and the same property
   cargo-mutants is supposed to give.

### Merge Order Rules

1. Merge independent green PRs first
2. Never use `gh pr merge --auto` on stacked PRs (rebase cancellation loop)
3. Rebase remaining PRs after each merge
4. Foundation PRs (config, lints) before dependent PRs

## State Management

| File | Purpose |
|------|---------|
| `plans/GOAP_STATE.md` | Canonical world state (YAML) |
| `plans/ACTIONS.md` | Action queue with preconditions/effects |
| `plans/GOAP_ORCHESTRATOR.md` | Per-session orchestrator state |
| `progress/LEARNINGS.md` | Cross-session learnings |
| `progress/PROGRESS.md` | Chronological progress notes |

After each action completes:
1. Mark action `status: complete` in `ACTIONS.md`
2. Update `action_last_completed` in `GOAP_STATE.md` (exactly once)
3. Add learnings to `progress/LEARNINGS.md` if new patterns discovered

## PR Triage

Before merging, run `./scripts/pr-triage.sh` to check:
- Merge conflict status (fix CONFLICTING first)
- CI pass/fail/pending counts
- Recommended merge order

## References

- `references/dependency-graph.md` — Issue dependency analysis
- `references/action-templates.md` — Reusable action patterns
- `references/wave-execution.md` — Multi-agent wave execution template
