# ADR-0020: Silent Data Loss on Load

## Status

Accepted (backfilled 2026-05-01)

## Context

Framework.load() semantics were unclear:
- Original: load() silently overwrites in-memory state
- Problem: Unexpected data loss
- Problem: No option to merge with existing

## Decision

Rename to **load_replace() and add load_merge()**.

**Rationale:**
- load_replace(): clear in-memory state first (explicit)
- load_merge(): append semantics (preserve existing)
- Default builder uses load_replace()
- Clear naming prevents confusion

## Consequences

### Positive
- Explicit load semantics
- Merge option preserves existing data
- No silent overwrites
- Backward compatible default

### Negative
- API change (load -> load_replace)
- Merge may create duplicates
- Requires caller to choose semantics

## Implementation

- Module: `src/framework.rs`
- Methods: load_replace(), load_merge()
- Default: FrameworkBuilder uses load_replace
- Merge: upsert semantics for duplicates

## Sources

- ACTIONS.md lines 206-216 (fix_framework_load_semantics action)
- src/framework.rs: load_replace/load_merge methods
- tests: framework lifecycle tests