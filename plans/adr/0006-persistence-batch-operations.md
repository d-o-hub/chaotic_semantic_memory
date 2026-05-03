# ADR-0006: Persistence Batch Operations

## Status

Accepted (backfilled 2026-05-01)

## Context

Bulk operations (import, initial load) require efficient database writes:
- Individual inserts: O(n) transactions
- High latency for thousands of concepts
- Per-row transaction overhead

## Decision

Add **batch operations** with transaction wrapping.

**Rationale:**
- Single BEGIN/COMMIT for all inserts
- Prepared statement reuse
- O(1) transaction overhead per batch
- 10-100x faster for bulk operations

## Consequences

### Positive
- Fast bulk import/export
- Atomic batch operations (rollback on failure)
- Reduced database round-trips
- Consistent API: save_concepts(), save_associations()

### Negative
- Larger transactions may hit limits
- Memory overhead for batch buffer
- Requires explicit batch API (not automatic)

## Implementation

- Module: `src/persistence.rs`
- Methods: `save_concepts(&[Concept])`, `save_associations(&[...])`
- Transaction: BEGIN/COMMIT wrapper
- Prepared statements: reset between executions (libsql requirement)

## Sources

- ACTIONS.md lines 325-337 (persistence_batch_ops action)
- MEMORY.md: "libSQL Prepared Statement Usage (v0.3.4, PR #103)"
- src/persistence.rs: batch save methods