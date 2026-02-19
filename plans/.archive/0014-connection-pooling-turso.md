# [ADR-0014] Connection Pooling for Remote Turso Databases

## Status
Accepted

## Context and Problem Statement
Current persistence uses per-operation connections from `Arc<Database>`. While cheap for local SQLite, remote Turso connections have higher latency. Creating a new connection per operation adds unnecessary overhead.

## Decision Drivers
- Turso remote connections have network latency
- Connection establishment overhead is significant for high-throughput workloads
- Must maintain per-operation model for local SQLite (no benefit to pooling)
- Must not break existing API

## Considered Options
1. **Per-operation (current)** - Simple, works for both local and remote
2. **deadpool-async** - Async connection pool, deadpool ecosystem
3. **bb8** - Tokio-based pool, very popular
4. **mobc** - Another async pool option

## Decision Outcome
Chosen option: **deadpool** for async connection pooling, gated for remote databases only

### Implementation Strategy
- Keep per-operation for local SQLite (no benefit, adds complexity)
- Use `deadpool::managed::Pool<Connection>` for Turso
- Configurable pool size (default: 10, max: 100)
- Health checks and automatic reconnection

### Positive Consequences
- Reduced latency for high-throughput Turso workloads
- Connection reuse amortizes establishment cost
- Automatic connection lifecycle management

### Negative Consequences
- Additional dependency (deadpool)
- More complex code path for remote databases
- Pool sizing requires tuning for specific workloads

## Links
- [deadpool crate](https://docs.rs/deadpool/)
- [Turso Best Practices](https://docs.turso.tech/sdk/rust)
