# ADR-0040: Async Lock Safety

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

Async lock safety issues:
- RwLock held across .await points
- load_replace/load_merge have lock scope issues
- import_json/import_binary hold locks during I/O
- Starvation risk during concurrent operations

## Decision

Implement **async lock safety**.

**Deliverables:**
- Restructure lock scopes to avoid holding RwLock across .await
- load_replace/load_merge: collect concept_ids, release lock, load associations, reacquire
- import operations: build concepts+associations while locked, release, then persist
- Eliminates starvation risk

## Consequences

### Positive
- No lock starvation
- Concurrent operations safe
- Proper async patterns
- Eliminates Send/Sync concerns

### Negative
- More complex lock management
- May require multiple lock/unlock cycles
- Performance overhead from lock churn

## Implementation

- Module: src/framework.rs, src/framework_ops.rs
- Pattern: lock -> collect -> release -> process -> lock -> write

## Sources

- ACTIONS.md lines 1222-1235 (fix_async_lock_safety action)
- ADR_REGISTRY.md: Async Lock Safety details
- src/framework.rs: lock scope restructuring