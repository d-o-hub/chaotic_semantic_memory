# ADR-0044: Memory Limits and Resource Governance

## Status

Proposed (backfilled 2026-05-01) - Wave 13

## Context

Resource governance gaps:
- No configurable concept limits
- Association counts unbounded
- Version history unlimited
- Metadata size unlimited
- Query cache size unspecified

## Decision

Implement **memory limits and resource governance**.

**Proposed Limits:**
- max_concepts: configurable (default: 100K, safe for memory)
- max_associations_per_concept: configurable (default: 1K)
- version_retention_limits: hard ceiling (1000 max)
- metadata_size_validation: 64KB limit per concept
- query_cache_size: configurable limits

## Consequences

### Positive
- Prevents resource exhaustion
- Configurable for workload
- Clear bounds for users
- Memory footprint predictable

### Negative
- Limits may reject valid operations
- Requires tuning for use case
- Configuration complexity

## Implementation

- Module: src/singularity.rs, src/framework.rs
- Config: SingularityConfig limits
- Validation: MemoryError::Capacity exceeded

## Sources

- ADR_REGISTRY.md: Memory Limits and Resource Governance
- W12 handoffs: memory budget analysis
- MEMORY.md: max_concepts, max_associations