# [ADR-0007] Similarity Search Optimization

## Status
Accepted

## Context and Problem Statement
`Singularity::find_similar()` currently:
1. Iterates sequentially over all concepts to compute cosine similarity
2. Sorts the entire result vector (O(n log n)) to find top-k
3. Uses `partial_cmp().unwrap()` which panics on NaN

For the target of 10 million concepts, a sequential brute-force scan is too slow, and full sorting wastes work when only top-k (typically 5-20) results are needed.

## Decision Drivers
* Must scale to 10M+ concepts with acceptable latency
* Must handle NaN similarities gracefully (no panics)
* top-k is typically small (5-20), so full sort is wasteful
* Must work with and without Rayon (WASM compatibility)
* Should not require a separate index structure for Phase 1

## Considered Options
1. **Rayon parallel scan + partial top-k selection**
2. **Approximate nearest neighbor index (LSH / HNSW)**
3. **Keep sequential + full sort (current)**

## Decision Outcome
Chosen option: **Rayon parallel scan + partial top-k** for Phase 1, with approximate indexing as a future Phase 2 trigger (when concept count exceeds ~200k and latency targets are missed).

### Implementation
```rust
pub fn find_similar(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
    let mut results: Vec<(String, f32)> = self.concepts
        .par_iter()  // rayon parallel
        .map(|(_, c)| (c.id.clone(), query.cosine_similarity(&c.vector)))
        .collect();

    if results.len() <= top_k {
        results.sort_by(|a, b| b.1.total_cmp(&a.1));  // NaN-safe
        return results;
    }

    // Partial selection: O(n) average instead of O(n log n)
    results.select_nth_unstable_by(top_k, |a, b| b.1.total_cmp(&a.1));
    results.truncate(top_k);
    results.sort_by(|a, b| b.1.total_cmp(&a.1));
    results
}
```

### Positive Consequences
* Linear scaling with CPU cores for similarity computation
* O(n) partial selection instead of O(n log n) full sort
* NaN-safe comparison via `total_cmp()`
* No new data structures or indices needed

### Negative Consequences
* Still O(n) scan — for 10M concepts this may be ~50-100ms
* Rayon overhead for small concept counts (< 1000)
* WASM fallback needed (sequential path)

## Future Trigger: Approximate Indexing
When all of the following are true:
- Concept count > 200,000
- Brute-force latency exceeds target (e.g., > 10ms)
- Query patterns are known (batch vs single)

Then consider ADR for multi-probe LSH on hypervectors:
- Hamming-based LSH is natural for binary hypervectors
- Multiple hash tables with random bit selections
- Pre-filter candidates, exact re-rank top-k

## Pros and Cons of the Options

### Rayon + partial top-k
* Good: Simple, no new data structures
* Good: Near-optimal for moderate concept counts
* Good: Exact results (no approximation error)
* Bad: Still linear scan

### Approximate indexing (LSH/HNSW)
* Good: Sub-linear query time
* Bad: Index maintenance cost on insert/delete
* Bad: Approximation error
* Bad: Premature for current scale

### Sequential + full sort (current)
* Good: Simplest code
* Bad: Single-threaded
* Bad: O(n log n) sort for top-5
* Bad: Panics on NaN
