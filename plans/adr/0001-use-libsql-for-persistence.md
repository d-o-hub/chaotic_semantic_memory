# ADR-0001: Use libSQL for Persistence

## Status

Accepted (backfilled 2026-05-01)

## Context

The Chaotic Semantic Memory system requires persistent storage for concepts and associations. Options considered:

1. **SQLite (rusqlite)** - Traditional SQLite bindings
2. **libSQL** - SQLite-compatible with async support and Turso remote capability
3. **PostgreSQL** - Full RDBMS but requires separate server
4. **Redis** - Key-value store, not suitable for complex associations

## Decision

Use **libSQL** for all persistence operations.

**Rationale:**
- Async-first API compatible with tokio runtime
- Local SQLite mode for development/testing
- Remote Turso mode for production deployments
- Full SQLite compatibility (no schema changes required)
- Built-in connection pooling support
- Single dependency covers all use cases

## Consequences

### Positive
- Zero-config local development (SQLite file)
- Seamless migration to Turso for production
- Async operations don't block the reservoir
- Transaction support for batch operations
- Per-operation connection model (no Arc<RwLock> needed)

### Negative
- libSQL is newer than rusqlite (fewer community examples)
- Turso requires network configuration for remote mode
- Connection overhead compared to in-memory only

## Implementation

- Module: `src/persistence.rs`
- Local: `Persistence::new_local(path)`
- Remote: `Persistence::new_turso(url, token)`
- Schema versioning with auto-migrations

## Sources

- ACTIONS.md lines 340-350 (persistence_connection_safety)
- GOAP_STATE.md: persistence schema version 5
- MEMORY.md: libSQL (SQLite/Turso) with auto-migrations