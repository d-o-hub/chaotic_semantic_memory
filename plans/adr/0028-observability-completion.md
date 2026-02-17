# ADR-0028: Observability Completion - Tracing and Metrics

## Status
- **Proposed**: 2026-02-17
- **Accepted**: Pending

## Context

The crate has structured logging via `tracing` in `framework.rs` and `persistence.rs`, but `singularity.rs` lacks instrumentation. The metrics system has basic counters but is missing:
- Cache hit/miss rates (critical for performance tuning)
- Reservoir operation metrics (key bottleneck)

This creates observability gaps that make production debugging difficult.

## Decision

### 1. Tracing Coverage Standards

All public methods in core modules must have `#[instrument]` attributes:

**Required in singularity.rs:**
- `inject()` - concept injection flow
- `get()` - concept retrieval
- `delete()` - concept deletion
- `find_similar()` - similarity search (already has timing, needs span)
- `associate()` - association creation
- `get_associations()` - association retrieval

**Tracing pattern:**
```rust
use tracing::{info, instrument};

#[instrument(skip(self), fields(concept_id = %id))]
pub fn inject(&mut self, concept: Concept) -> Result<()> {
    info!("Injecting concept");
    // ... logic
}
```

### 2. Metrics Expansion

Add the following metrics:

**Cache Metrics (singularity.rs):**
- `cache_hits_total` (counter): LRU cache hits
- `cache_misses_total` (counter): LRU cache misses
- `cache_evictions_total` (counter): Cache evictions

**Reservoir Metrics (reservoir.rs):**
- `reservoir_steps_total` (counter): Total step operations
- `reservoir_step_latency_us` (histogram): Step latency distribution
- `reservoir_nodes_active` (gauge): Current reservoir size

### 3. Performance Impact

- Tracing: ~50ns per span (negligible)
- Metrics: Atomic increments (~5ns)
- Total overhead: <0.1% for typical workloads

## Consequences

### Positive
- Complete request tracing across all modules
- Cache hit rates visible for capacity planning
- Reservoir performance metrics for optimization
- Production debugging capabilities

### Negative
- Additional dependencies: None (tracing/metrics already present)
- Code complexity: +50-80 lines
- Runtime overhead: Minimal but measurable

### Alternative Considered
**Use OpenTelemetry instead of tracing**: Rejected - tracing is sufficient for current needs, OpenTelemetry adds unnecessary complexity.

## Implementation Plan

1. Add tracing to singularity.rs (1 day)
2. Add cache metrics to LRU operations (1 day)
3. Add reservoir metrics to step() method (1 day)
4. Verify with `RUST_LOG=debug cargo test` (1 day)

## Compliance

- **500 LOC limit**: Yes, additions minimal
- **No hardcoded settings**: Metrics use constants for bucket sizes
- **WASM compatible**: Tracing works in WASM, metrics conditionally compiled

## References

- Analysis artifact: `plans/handoffs/analysis_group_c_docs.md`
- Existing tracing: `src/framework.rs` (11 `#[instrument]` attributes)
- Current metrics: `src/framework.rs`, `src/singularity.rs`
