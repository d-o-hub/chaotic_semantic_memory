# Wave Execution Template

Multi-agent wave execution pattern for GOAP orchestrator.

## Wave Structure

A wave is a group of independent actions that can execute in parallel.
Each action gets its own feature branch and PR.

```
Wave N
├─ Action A (cost 3, independent)     → agent-1
├─ Action B (cost 5, independent)     → agent-2
├─ Action C (cost 3, depends on A)    → agent-3 (after A merges)
└─ Action D (cost 2, depends on B,C)  → agent-4 (after B,C merge)
```

## Dispatch Protocol

### Step 1: Explore Phase (parallel)

Spawn explore agents to audit affected files before implementation:

```
explore-1: "Find all files affected by action A. Report file paths,
            line numbers, and current state."
explore-2: "Find all files affected by action B. Report file paths,
            line numbers, and current state."
```

### Step 2: Implement Phase (parallel)

Spawn general agents for independent actions:

```
general-1: "Implement action A on branch feat/action-a.
            Context from explore-1: [findings]"
general-2: "Implement action B on branch feat/action-b.
            Context from explore-2: [findings]"
```

### Step 3: Validate Phase (per-agent)

Each agent runs before completing:
- `cargo check --all-features --quiet`
- `cargo test --all-features --quiet`
- `cargo clippy --quiet -- -D warnings`
- `cargo fmt --check --quiet`

### Step 4: PR Phase (sequential)

Create PRs in dependency order:
1. Independent PRs first (A, B)
2. Wait for CI green
3. Merge in order: A → rebase C → merge C
4. Dependent PRs after dependencies merge

## Merge Order Algorithm

```
function mergeOrder(actions):
    ready = actions where all preconditions are met
    merged = []

    while ready is not empty:
        # Merge independent actions first (no dependencies on unmerged)
        independent = ready where deps ⊆ merged
        for action in independent:
            wait for CI green
            merge action PR
            merged.add(action)

        # Rebase remaining actions
        for action in ready - independent:
            rebase onto main

        # Recalculate ready set
        ready = actions where all preconditions are met and not in merged

    return merged
```

## Error Recovery

If an agent fails:
1. Log the error in `progress/LEARNINGS.md`
2. Create a new task for manual review
3. Continue with independent actions
4. Block dependent actions until failure is resolved

## Concurrency Limits

- Max 4 parallel agents (GitHub Actions concurrency awareness)
- Each agent gets its own worktree (no file conflicts)
- Shared state files (GOAP_STATE.md, ACTIONS.md) are updated only at merge time
