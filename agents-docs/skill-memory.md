# Skill Memory

This repository uses two memory-oriented skills instead of one broad skill.

## Skill Split

- `skill-memory-internal`: daily dogfooding memory for agent development workflows.
- `memory-lifecycle-verification`: portable verification for save/load/archive/delete across files and DB entries.

## When to Use Which

### `skill-memory-internal`

Use during implementation, debugging, planning, and test loops to store and recall operational context.

Path:

- `.agents/skills/skill-memory-internal/SKILL.md`

### `memory-lifecycle-verification`

Use before release or when onboarding memory behavior into another codebase. This skill is the portability and correctness contract.

Path:

- `.agents/skills/memory-lifecycle-verification/SKILL.md`

## Shared Configuration

```yaml
memory:
  enabled: true
  database: ".agents/csm-memory/skill-memory.db"
  namespace_prefix: "skill"
```

## Internal Memory Quick Usage

```bash
export CSM_MEMORY_DB=".agents/csm-memory/skill-memory.db"

# Save
csm --database "$CSM_MEMORY_DB" inject \
  "skill::impl::decision::$(date +%s)" \
  --metadata '{"operation":"decision","result":"accepted"}'

# Load
csm --database "$CSM_MEMORY_DB" probe "decision accepted" -k 5 --output-format json

# Associate
csm --database "$CSM_MEMORY_DB" associate \
  "skill::impl::decision::123" "skill::test::validation::123" -s 0.9
```

## Lifecycle Verification Minimum Contract

Every verification run must prove all four operations:

- `save`: data persisted and discoverable
- `load`: export/import roundtrip preserves IDs and metadata
- `archive`: archived state is recorded and auditable
- `delete`: deleted/tombstoned entries are no longer active and leave no orphans

Reference artifacts:

- `.agents/skills/memory-lifecycle-verification/references/VALIDATION_CHECKLIST.md`
- `.agents/skills/memory-lifecycle-verification/references/sql_checks.sql`

## Why This Split

- Keeps day-to-day memory use simple for internal agent work.
- Makes lifecycle verification reusable in other repositories.
- Ensures file + database behavior is testable with explicit evidence.
