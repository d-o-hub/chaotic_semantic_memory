# ADR-0083: Memory Lifecycle Verification & Export Format Contract

## Status

Accepted

## Context and Problem Statement

The `memory-lifecycle-verification` skill was dogfooded against the
codebase (2026-05-18) to verify that load/save/archive/delete CRUD
operations work correctly in both file and database backends.

The verification revealed four gaps:

1. **sql_checks.sql table names**: The skill's reference SQL queries
   used bare table names (`concepts`, `associations`) but the crate
   uses `csm_`-prefixed names (`csm_concepts`, `csm_associations`).
   Column names were also wrong (`source_id`/`target_id` instead of
   `from_id`/`to_id`).

2. **VALIDATION_CHECKLIST.md stale**: All checkboxes were `[ ]` with
   no way to trace which verification run covered which items.

3. **GOAP_STATE.md stale flags**: `archive_phase_skipped: true` and
   `delete_phase_skipped: true` from the 2026-04-30 verification were
   no longer accurate — `csm delete` exists and archive markers work.

4. **Export JSON serialization format**: The `csm export` command
   serializes associations as position-dependent arrays:
   `["from_id", "to_id", strength]` rather than named objects.
   This undocumented contract surprises consumers who expect
   structured JSON.

## Decision Drivers

- Cross-repository portability: the skill targets multiple codebases
- Self-documenting verification: checkboxes must record actual runs
- Accurate GOAP state: stale flags mislead the planner
- Backward compatibility: changing the export format breaks existing
  consumers (CI pipelines, import scripts, manual inspection)

## Considered Options

### Option 1: Change export format to named objects + document

Change `csm export` to emit objects:

```json
{"from_id": "alpha", "to_id": "beta", "strength": 0.9}
```

Update documentation and import parser to handle both formats
during a deprecation window.

- Good, because self-documenting JSON is more maintainable
- Bad, because it breaks every existing export artifact and consumer
- Bad, because `csm import` must maintain backward compat for
  existing export files

### Option 2: Keep array format, document the contract explicitly

Document that associations serialize as
`[from_id: string, to_id: string, strength: f64]` tuples.
Accept this as the canonical format.

- Good, because zero breakage — all existing exports remain valid
- Good, because `csm import` already handles this format (no change)
- Bad, because array-of-tuples is less self-documenting than objects

### Option 3: Add a named-object variant via `--output-format`

Let `csm export --output-format json-pretty` (or a future
`--associations-as-objects` flag) emit named objects. Keep
the default as arrays for backward compat.

- Good, because no breakage by default
- Good, because consumers who want objects can opt in
- Bad, because it increases the API surface for a minor ergonomic
  concern
- Bad, because both formats must be maintained indefinitely

## Decision Outcome

Chosen option: **Option 2 — keep array format, document the contract**.

Rationale: The export format is an internal serialization detail, not
a user-facing API. The array-of-tuples format is compact,
unambiguous (positions are never reordered), and already handled by
both `csm export` and `csm import`. Changing it would break every
existing export artifact for no functional gain. Documenting it in
this ADR and in the VALIDATION_CHECKLIST.md provides the traceability
needed without compatibility risk.

### Positive Consequences

- Zero breakage of existing export files and CI pipelines
- No import parser changes needed
- Low implementation cost (documentation-only decision)

### Negative Consequences

- Array-of-tuples is less readable for manual inspection
- External consumers must hardcode position-based parsing

## Pros and Cons of the Options

### Option 1: Named objects

- Good, because readable `{"from_id":"alpha","to_id":"beta","strength":0.9}`
- Bad, because breaks every `csm export` artifact in the wild
- Bad, because `csm import` must support both formats forever

### Option 2: Keep arrays, document (chosen)

- Good, because zero breakage
- Good, because import parser unchanged
- Bad, because manual readers must know positions

### Option 3: Optional named-object variant

- Good, because opt-in for consumers who want objects
- Bad, because two serialization paths to maintain
- Bad, because format negotiation adds complexity for rare need

## Follow-up Actions

- [x] `sql_checks.sql` updated with correct table/column names
- [x] VALIDATION_CHECKLIST.md marked for 2026-05-18 run
- [x] GOAP_STATE stale archive/delete flags annotated
- [x] LEARNINGS.md updated with lifecycle verification patterns
