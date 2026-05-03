# ADR-0047: Security & Performance Hardening

## Status

Accepted (backfilled 2026-05-01) - Wave 13 Complete

## Context

Security and performance issues identified by analysis swarm:
- Bincode deserialization unlimited (OOM DoS risk)
- Error source attributes missing
- Production expect() calls
- Cache uses Mutex (not RwLock)
- Path validation missing

## Decision

Implement **security & performance hardening**.

**Deliverables:**
- Bincode size limits (100MB max)
- Error source attributes (#[source] chains)
- Replace expect() with proper Result propagation
- Cache Mutex -> RwLock (concurrent reads)
- Path validation for file operations
- Reservoir and persistence tracing

## Consequences

### Positive
- OOM attacks prevented
- Error chains for debugging
- No panic on reservoir access
- Concurrent cache reads
- Path traversal blocked
- Full tracing coverage

### Negative
- Large imports rejected
- Error handling more complex
- RwLock overhead vs Mutex
- Path restrictions

## Implementation

- Module: src/framework_ops.rs, src/error.rs, src/singularity.rs
- Pattern: bincode limits, #[source], RwLock, path validation

## Sources

- ACTIONS.md lines 1858-1958 (Phase 27 actions)
- Git: feat(security): implement ADR-0047
- W12 analysis handoffs