# [ADR-0006] Persistence Batch Operations

## Status
Proposed

## Context and Problem Statement
The current persistence layer performs one database round-trip per concept save/load. When persisting many concepts (e.g., bulk import, framework shutdown), this results in N sequential round-trips:
- Local SQLite: ~0.1ms per INSERT → 100ms for 1000 concepts
- Remote Turso: ~5-20ms per round-trip → 5-20 seconds for 1000 concepts
- No transaction batching means no atomicity guarantees for bulk operations

## Decision Drivers
* Bulk import of concepts must be efficient (< 1 second for 1000 concepts locally)
* Remote Turso round-trip cost must be amortized
* Must maintain atomicity — all-or-nothing for bulk operations
* Must not exceed 500 LOC limit for persistence.rs (currently 391)

## Considered Options
1. **Transaction-wrapped batch with prepared statements**
2. **`execute_batch()` with raw SQL concatenation**
3. **Keep current per-concept approach**

## Decision Outcome
Chosen option: **Transaction-wrapped batch with prepared statements**, because:
- BEGIN/COMMIT wrapping gives atomicity
- Prepared statement reuse within a transaction reduces SQL parsing overhead (~50% faster per the project learnings)
- Clean API: `save_concepts(&[Concept])`, `save_associations(&[(from, to, strength)])`
- Stays within LOC budget (~30 LOC added)

### Positive Consequences
* 10-100x faster bulk operations (amortized commit + prepared statements)
* Atomic bulk operations (all-or-nothing)
* Clean batch API for framework-level bulk persist
* Reduces Turso round-trips from N to 1 per batch

### Negative Consequences
* Slightly more complex persistence code
* Large batches may hold write locks longer (mitigated: configurable batch size)

## API Design
```rust
impl Persistence {
    /// Save multiple concepts in a single transaction
    pub async fn save_concepts(&self, concepts: &[Concept]) -> Result<()>;

    /// Save multiple associations in a single transaction
    pub async fn save_associations(&self, assocs: &[(String, String, f32)]) -> Result<()>;
}
```

## Pros and Cons of the Options

### Transaction-wrapped batch
* Good: Atomic, fast, clean API
* Good: Prepared statement reuse
* Bad: Slightly more code

### execute_batch() with raw SQL
* Good: Single call to libsql
* Bad: No parameterized queries (SQL injection risk)
* Bad: String concatenation for values

### Per-concept (current)
* Good: Simple code
* Bad: O(N) round-trips
* Bad: No atomicity
