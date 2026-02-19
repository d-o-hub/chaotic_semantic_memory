# [ADR-0005] Persistence Connection Model

## Status
Accepted

## Context and Problem Statement
The current `Persistence` struct wraps a single `libsql::Connection` behind `Arc<RwLock<Connection>>`. This has several issues:
- `libsql::Connection` may not be `Send` across all backends, creating potential UB or panics under multi-threaded tokio runtimes
- A single connection behind RwLock serializes all database operations, preventing concurrent reads
- Write lock contention during `save_concept()` blocks all readers (`load_concept()`, `load_all_concepts()`)

## Decision Drivers
* Must be safe under tokio multi-threaded runtime
* Must support concurrent read operations
* Must maintain correctness for write operations (no lost updates)
* Must work with both local SQLite and remote Turso backends
* Should not add significant complexity

## Considered Options
1. **Per-operation connection from `Arc<Database>`** — create a fresh connection for each DB call
2. **Connection pool** — maintain a pool of N connections, checkout/return
3. **Keep current `Arc<RwLock<Connection>>`** — status quo with documented limitations

## Decision Outcome
Chosen option: **Per-operation connection from `Arc<Database>`**, because:
- `libsql::Database` is `Send + Sync` and designed to create connections
- Connection creation is cheap for local SQLite
- Eliminates Send/Sync concerns entirely — each connection is used on one task
- Concurrent reads become naturally parallel
- Writes are serialized by SQLite's own locking (correct behavior)
- Simplest implementation with fewest assumptions about libsql internals

### Positive Consequences
* Eliminates potential Send/Sync violations
* Concurrent reads are no longer blocked by write lock
* Simpler code — no RwLock management
* Each async operation is self-contained

### Negative Consequences
* Connection creation overhead per operation (mitigated: SQLite connections are cheap)
* No prepared statement caching across operations (mitigated: batch operations handle hot paths)
* For remote Turso, connection creation may have higher latency (mitigated: batch APIs reduce call count)

## Pros and Cons of the Options

### Per-operation connection
* Good: Inherently safe (no shared mutable state)
* Good: Concurrent reads
* Good: Simple implementation
* Bad: Connection creation cost per call

### Connection pool
* Good: Amortizes connection creation
* Good: Bounded resource usage
* Bad: Added complexity (pool management, checkout timeouts)
* Bad: Overkill for typical usage patterns

### Current Arc<RwLock<Connection>>
* Good: No code changes needed
* Bad: Potential Send/Sync violations
* Bad: Serializes all DB operations
* Bad: Write lock blocks readers
