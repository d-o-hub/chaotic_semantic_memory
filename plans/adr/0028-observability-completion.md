# ADR-0028: Observability Completion

## Status

Accepted (backfilled 2026-05-01) - Wave 7 Complete

## Context

Observability gaps identified:
- Singularity methods not traced
- Cache metrics not collected
- Reservoir metrics missing

## Decision

Complete **observability instrumentation**.

**Deliverables:**
- #[instrument] spans on Singularity methods (inject, get, delete, find_similar)
- Cache hit/miss counters (cache_hits_total, cache_misses_total, cache_evictions_total)
- Reservoir metrics (reservoir_steps_total, reservoir_step_latency_us, reservoir_nodes_active)

## Consequences

### Positive
- Full tracing across core modules
- Cache performance visible
- Reservoir health monitored
- Production debugging enabled

### Negative
- Tracing overhead in hot paths
- Metrics collection memory overhead
- WASM requires cfg gating

## Implementation

- Module: src/singularity.rs (tracing), src/reservoir.rs (metrics)
- Metrics: Prometheus-compatible counters/histograms
- Export: metrics endpoint in framework

## Sources

- ACTIONS.md lines 1001-1044 (Phase 14 actions)
- ADR_REGISTRY.md: Wave 7 Active ADRs
- W1_C_to_All_tracing_conventions.md