---
name: skill-memory-internal
description: Internal dogfooding skill for storing and recalling engineering context through the csm CLI during day-to-day development.
---

# Internal Skill Memory (Dogfooding)

Use this skill when you want agents to persist operational context while implementing, fixing, testing, or planning.

## Purpose

- Keep short-lived engineering memory in a local CSM database.
- Record decisions, failures, and successful patterns in a reusable format.
- Improve continuity across multi-step tasks without external dependencies.

## Scope

- Internal development workflows in this repository.
- Fast write/read cycles during coding sessions.
- Not intended as a portability or compliance skill.

## Required Environment

```bash
export CSM_MEMORY_DB=".agents/csm-memory/skill-memory.db"
mkdir -p "$(dirname "$CSM_MEMORY_DB")"
```

## Canonical ID Format

Use stable IDs so later probes can find related entries:

```text
skill::<skill_name>::<category>::<slug-or-timestamp>
```

Examples:

- `skill::impl::refactor::2026-04-12T08-22-10Z`
- `skill::fix::error-resolution::merge-conflict-overwrite`

## Core Operations

### Remember

```bash
csm --database "$CSM_MEMORY_DB" inject \
  "skill::fix::error-resolution::load-merge-overwrite" \
  --metadata '{"operation":"error_resolution","context":"load_merge conflict policy","result":"skip-on-conflict"}'
```

### Recall

```bash
csm --database "$CSM_MEMORY_DB" probe "load merge overwrite policy" -k 5 --output-format json
```

### Link patterns

```bash
csm --database "$CSM_MEMORY_DB" associate \
  "skill::fix::error-resolution::load-merge-overwrite" \
  "skill::test::regression::load-merge-no-overwrite" \
  -s 0.9
```

## Operational Rules

- Write only concrete observations and outcomes.
- Include enough metadata to replay context (`operation`, `context`, `result`, `timestamp`).
- Keep this memory local and non-sensitive.

## Quick Validation

```bash
# write
csm --database "$CSM_MEMORY_DB" inject "skill::internal::smoke::$(date +%s)" --metadata '{"smoke":true}'

# read
csm --database "$CSM_MEMORY_DB" probe "smoke" -k 1 --output-format json
```

If both commands succeed and return at least one result, internal memory is operational.
