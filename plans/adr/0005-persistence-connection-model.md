# ADR-0005: Persistence Connection Model

## Status

Accepted (backfilled 2026-05-01)

## Context

Database connection management for async operations:
- Original design: Arc<RwLock<Connection>> for shared access
- Problem: Send/Sync safety issues with tokio
- Problem: Lock contention during concurrent reads

## Decision

Use **per-operation connection model** from Arc<Database>.

**Rationale:**
- Each operation creates its own Connection from Database
- No lock contention (connections are independent)
- Connection creation is cheap for local SQLite
- Safe for concurrent async operations
- Compatible with Turso remote mode

## Consequences

### Positive
- Eliminates Send/Sync safety concerns
- Enables concurrent read operations
- Simple error handling per-operation
- Works for both local and remote modes

### Negative
- Connection overhead per operation
- No shared prepared statement cache
- Requires Arc<Database> instead of Arc<Connection>

## Implementation

- Module: `src/persistence.rs`
- Pattern: `db.connect().await?` per operation
- Connection lifetime: single operation scope

## Sources

- ACTIONS.md lines 338-350 (persistence_connection_safety action)
- src/persistence.rs: connect() method
- MEMORY.md: libSQL connection model