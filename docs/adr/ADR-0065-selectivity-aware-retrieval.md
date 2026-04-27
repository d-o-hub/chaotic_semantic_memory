# ADR-0065: Selectivity-Aware Filtered Retrieval

## Status

Accepted

## Context

The current `find_similar_filtered()` in `singularity_ext.rs` always pre-filters concepts regardless of selectivity:

1. Scan all concepts, filter by metadata predicate
2. Score only matching candidates

This is optimal when few concepts match (low selectivity), but wasteful when most concepts match (high selectivity) — we scan all concepts just to discard few.

Paper: Amanbayev et al., "Filtered ANN Search", arXiv:2602.11443, Feb 2026

Key insight: Optimal strategy depends on **selectivity ratio** = matching_count / total_count:
- **Low selectivity (< 0.3)**: Pre-filter is optimal (few candidates to score)
- **Medium selectivity (0.3-0.8)**: Bucket/graph candidates → post-filter → score
- **High selectivity (≥ 0.8)**: Standard similarity search → post-filter results

## Decision

Route `find_similar_filtered()` to different strategies based on computed selectivity ratio:
1. Compute selectivity before building candidate list
2. Route to optimal strategy based on thresholds
3. Expose selectivity ratio and strategy used in `RetrievalStats`

## Implementation

### Strategy Routing

```rust
let matching_count = concepts.values().filter(|c| filter.matches(&c.metadata)).count();
let selectivity = matching_count as f32 / concepts.len() as f32;

if selectivity < 0.3 {
    // PreFilter: current path (already optimal)
} else if selectivity < 0.8 {
    // BucketPostFilter: bucket candidates → post-filter → score
} else {
    // ScanPostFilter: find_similar() → post-filter results
}
```

### New Types

```rust
pub enum FilterStrategy {
    PreFilter,        // Pre-filter candidates, then score
    BucketPostFilter, // Bucket candidates, score, post-filter
    ScanPostFilter,   // Full scan, score, post-filter
}
```

### RetrievalStats Extensions

```rust
pub struct RetrievalStats {
    // ... existing fields ...
    pub selectivity_ratio: f32,
    pub filter_strategy: FilterStrategy,
}
```

## Consequences

### Benefits
- Optimal routing based on actual selectivity
- Exposed metrics for observability
- Configurable thresholds via RetrievalConfig

### Costs
- One extra count pass over concepts to estimate selectivity
- Bucket path may include non-matching candidates (filtered after scoring)

### LOC Impact
- singularity_ext.rs at 144 LOC; implementation adds ~30 lines (well under 500)
- singularity_retrieval.rs at 302 LOC; FilterStrategy enum adds ~10 lines

## References

- Amanbayev et al., "Filtered ANN Search", arXiv:2602.11443, Feb 2026
- GOAP_ACTION: write_adr_selectivity_aware_retrieval, implement_selectivity_aware_retrieval