# ADR-0056: Performance Follow-up Priorities

## Status

Accepted (backfilled 2026-05-01) - Wave 17 Complete

## Context

Performance optimization priorities:
- Reservoir step latency improvements
- BM25 search optimization
- SparseWeights memory optimization
- Similarity search improvements

## Decision

Document **performance follow-up priorities**.

**Deliverables:**
- SparseWeights AoS optimization (31.5% speedup)
- BM25 OOV term skipping
- Reservoir lazy projection
- Similarity fused integer scoring

## Consequences

### Positive
- Clear optimization priorities
- Measured performance gains
- Optimization methodology documented
- Benchmark baselines set

### Negative
- Optimization effort required
- May need hardware-specific tuning
- Trade-offs documented

## Implementation

- Phase: 48 (Wave 17)
- Git: perf(reservoir): optimize sparse weights with fused AoS (#135)
- Benchmark: reservoir_step_50k ~129us

## Sources

- Git: docs(planning): add ADR-0056 follow-up priorities
- ADR_REGISTRY.md: Performance Follow-up Priorities
- MEMORY.md: performance benchmarks