# [ADR-0035] Cache Memory Guardrails for Similarity Query Cache

## Status
Proposed

## Context and Problem Statement
The similarity query cache (`QueryCache` in `src/singularity.rs`) stores `Arc<[(String, f32)]>` per query. With default configuration:
- Cache capacity: 1000 entries (DEFAULT_CONCEPT_CACHE_SIZE)
- Max top_k per probe: 10,000 (DEFAULT_MAX_PROBE_TOP_K)

In the worst case, caching 1000 queries × 10,000 results × owned String IDs can consume significant memory. For a concept store with long IDs (~64 bytes average), this approaches `1000 × 10000 × (64 + 4) ≈ 680 MB` of cached data.

## Decision Drivers
- Memory-constrained environments (WASM, embedded) cannot tolerate unbounded cache growth
- The cache is a performance optimization, not a correctness requirement
- Default configuration should be safe without manual tuning

## Considered Options
- Option A: Skip caching when `top_k` exceeds a threshold (e.g., 100)
- Option B: Reduce default cache capacity to 128 and add max_cached_top_k config
- Option C: Add memory-budget-based eviction

## Decision Outcome
Chosen option: "Option B — Reduce defaults and add `max_cached_top_k` config", because it provides safe defaults while allowing power users to tune for their workload.

### Implementation
1. Reduce `DEFAULT_CONCEPT_CACHE_SIZE` from 1000 to 128
2. Add `max_cached_top_k: usize` to `SingularityConfig` (default: 100)
3. In `find_similar_cached()`: if `top_k > config.max_cached_top_k`, bypass cache and compute directly
4. Expose `with_max_cached_top_k()` on `FrameworkBuilder`

### Positive Consequences
- Safe memory usage by default (~128 × 100 × 72 ≈ 920 KB worst case)
- Large-top_k queries still work, just bypass cache
- Power users can increase both limits for large-scale use

### Negative Consequences
- Existing users with top_k > 100 will see cache misses (easily tuned)
- One additional config field to document
