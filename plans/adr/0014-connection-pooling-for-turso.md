# ADR-0014: Connection Pooling for Turso

## Status

Accepted (backfilled 2026-05-01)

## Context

Remote Turso connections have higher latency:
- Per-operation model works for local SQLite
- Remote: connection establishment overhead
- Problem: Latency adds to every operation

## Decision

Implement **connection pooling** for remote Turso.

**Rationale:**
- Use deadpool or bb8 for async pool
- Configurable pool size (default: 10)
- Health checks and connection recycling
- Keep per-operation model for local SQLite

## Consequences

### Positive
- Reduced latency for remote operations
- Connection reuse amortizes overhead
- Health checks prevent stale connections
- Local SQLite unchanged (simple model)

### Negative
- Pool configuration complexity
- Additional dependency (deadpool/bb8)
- Pool exhaustion possible under load

## Implementation

- Module: `src/persistence.rs`
- Pool: deadpool-libsql or bb8-libsql
- Local: single connection per operation (unchanged)
- Remote: pool with configurable size

## Sources

- ACTIONS.md lines 567-581 (add_connection_pooling action)
- src/persistence.rs: new_turso_with_pool()
- MEMORY.md: Turso connection profile