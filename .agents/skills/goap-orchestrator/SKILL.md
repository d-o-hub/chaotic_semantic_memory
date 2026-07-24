---
name: goap-orchestrator
description: GOAP-based orchestrator for managing GitHub issues, creating action plans, and executing workspace operations with branch/PR workflow.
---

# GOAP Orchestrator

Orchestrate complex multi-issue tasks using GOAP planning with GitHub integration.

## Workflow

1. **Scan Issues**: Read all open GitHub issues via `gh issue list`
2. **Build Dependency Graph**: Analyze issue relationships and prerequisites
3. **Create GOAP Plan**: Generate ordered action list with preconditions/effects
4. **Execute Actions**: Implement changes in dependency order
5. **Branch & PR**: Create feature branches, atomic commits, PRs
6. **Verify CI**: Ensure all GitHub Actions pass before merging

## Commands

```bash
# Scan and plan
./scripts/goap-orchestrator.sh scan          # List all open issues
./scripts/goap-orchestrator.sh plan          # Generate GOAP action plan
./scripts/goap-orchestrator.sh execute       # Execute next action in plan
./scripts/goap-orchestrator.sh verify        # Check CI status
```

## State Management

- Plan state: `plans/GOAP_ORCHESTRATOR.md`
- Issue tracking: `gh issue list` (GitHub as source of truth)
- Branch strategy: `feat/<scope>-<description>`

## References

- `references/dependency-graph.md` - Issue dependency analysis
- `references/action-templates.md` - Reusable action patterns
