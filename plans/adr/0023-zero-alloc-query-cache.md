# ADR-0023: Zero-Alloc Query Cache

## Status

Accepted (backfilled 2026-05-01)

## Context

Repeated similarity queries have allocation overhead:
- Original: Cache key uses HVec10240::to_bytes() allocation
- Original: Cache hit clones Vec<(String, f32)>
- Problem: Allocations slow repeated queries

## Decision

Implement **zero-allocation query cache**.

**Rationale:**
- Hash cache keys from HVec10240 words (no to_bytes())
- Store cached results as Arc<[(String, f32)]>
- Avoid cloning Vec on cache hits
- expose find_similar_cached() and probe_batch_cached()

## Consequences

### Positive
- Reduced allocation overhead
- Faster repeated queries
- Arc sharing avoids clones
- Cache bypass when top_k > max_cached

### Negative
- Cache memory overhead
- Cache invalidation complexity
- Requires RwLock for concurrent access

## Implementation

- Module: `src/singularity.rs`, `src/framework.rs`
- Cache: LRU with zero-alloc keys
- Key: hash from words array
- Value: Arc<[(String, f32)]>

## Sources

- ACTIONS.md lines 855-867 (enable_zero_alloc_query_cache action)
- src/singularity.rs: find_similar_cached
- ADR-0035: cache memory guardrails