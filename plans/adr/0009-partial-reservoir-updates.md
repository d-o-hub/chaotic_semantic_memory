# ADR-0009: Partial Reservoir Updates

## Status

Accepted (backfilled 2026-05-01)

## Context

Reservoir step() updates all 50k nodes every iteration:
- Full update: O(n*k) = 50k * 64 = 3.2M operations
- Problem: High latency for real-time use
- Problem: Unnecessary for partial input

## Decision

Use **partitioned updates** with stride-based rotation.

**Rationale:**
- Update subset of nodes per step
- Stride 32, rotating phase
- Cached input projection reuse
- O(partition*k) instead of O(n*k)

## Consequences

### Positive
- Reduced per-step latency
- Local-neighborhood sparse connectivity
- Cache-friendly access pattern
- ~88us median latency (target <100us met)

### Negative
- Partial update requires state tracking
- Phase rotation complexity
- May miss some activations temporarily

## Implementation

- Module: `src/reservoir.rs`
- Pattern: Partitioned updates, stride 32
- Optimization: Cached input projection
- Benchmark: reservoir_step_50k ~129us (with AoS optimization)

## Sources

- ACTIONS.md lines 447-467 (optimize_reservoir_step_latency action)
- MEMORY.md: reservoir_step_50k ~129us benchmark
- PR #135: SparseWeights AoS optimization