# ADR-0007: Similarity Search Optimization

## Status

Accepted (backfilled 2026-05-01)

## Context

find_similar() searches all concepts for top-k matches:
- Original: sequential iteration O(n)
- Problem: Slow for thousands of concepts
- Problem: Full sort when only top-k needed

## Decision

Use **parallel search with partial sort**.

**Rationale:**
- Rayon par_iter() for parallel iteration
- select_nth_unstable_by for partial top-k (no full sort)
- total_cmp() for NaN-safe comparison
- O(n) parallel with O(k) extraction

## Consequences

### Positive
- 4-8x speedup on multi-core systems
- No unnecessary full sort
- NaN-safe floating point comparison
- Configurable top_k (bypass cache when >100)

### Negative
- Rayon overhead for small n (<100)
- WASM requires sequential fallback (see ADR-0008)
- Par_iter requires Send/Sync bounds

## Implementation

- Module: `src/singularity.rs`
- Method: `find_similar()` with par_iter()
- Extraction: `select_nth_unstable_by`
- WASM fallback: sequential iteration

## Sources

- ACTIONS.md lines 286-298 (parallel_similarity_search action)
- src/singularity.rs: find_similar implementation
- MEMORY.md: Cache Memory Guardrails (ADR-0035)