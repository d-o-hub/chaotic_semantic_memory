# ADR-0035: Cache Memory Guardrails

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

Cache memory concerns:
- DEFAULT_CONCEPT_CACHE_SIZE = 1000 (too large for small systems)
- Cache accumulates with no bypass mechanism
- Large top_k queries fill cache unnecessarily

## Decision

Implement **cache memory guardrails**.

**Deliverables:**
- Reduce DEFAULT_CONCEPT_CACHE_SIZE to 128
- Add max_cached_top_k (default: 100)
- Bypass cache when top_k > max_cached_top_k
- Expose with_max_cached_top_k() on FrameworkBuilder

## Consequences

### Positive
- Smaller memory footprint
- Cache bypass for large queries
- Configurable limits
- Prevents cache bloat

### Negative
- Smaller cache may miss more
- Large queries slower (no cache)
- Requires tuning for workload

## Implementation

- Module: src/singularity.rs, src/framework.rs
- Config: SingularityConfig.max_cached_top_k
- Logic: bypass when top_k exceeds limit

## Sources

- ACTIONS.md lines 1343-1357 (add_cache_memory_guardrails action)
- ADR_REGISTRY.md: Cache Memory Guardrails
- src/singularity.rs: cache bypass logic