# Specialist Handoff Contract

Each specialist handoff must include:

1. Objective
2. Input files/context
3. Constraints
4. Expected deliverables
5. Validation commands
6. Exit criteria

## Specialist Lanes

### architecture-agent
- Creates architecture updates and diagrams.
- Produces ADR impact notes.

### implementation-agent
- Implements required code changes.
- Ensures no TODO/mock placeholders.

### test-agent
- Adds/updates tests.
- Confirms deterministic pass criteria.

### performance-agent
- Runs/updates benchmarks.
- Flags regressions with reproducible command output.

### persistence-agent
- Validates durability and restore paths.
- Verifies schema and checkpoint behavior.

### wasm-agent
- Verifies wasm compatibility and edge behavior.

### release-agent
- Ensures commit, PR metadata, and completion checklist quality.
