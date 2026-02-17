---
name: swarm-performance
description: "SIMD optimization, connection pooling, batch APIs, and caching. Use when improving throughput or reducing latency."
---

# Swarm: Performance

## Workflow
1. Profile current performance with `cargo bench --bench benchmark`
2. Identify hot path from flamegraph or benchmark results
3. Implement optimization behind feature flag if experimental
4. Benchmark before/after with criterion baseline
5. Ensure SIMD has scalar fallback for non-SIMD targets
6. Run all gates before claiming improvement

## SIMD Implementation

```rust
#[cfg(feature = "simd")]
use std::simd::u128x2;

pub fn cosine_similarity_simd(&self, other: &Self) -> f32 {
    // Use u128x2 for parallel operations
    // Fall back to scalar for WASM/non-SIMD targets
}
```

## Connection Pooling

Use `deadpool` for async connection pooling, gated for remote Turso only.
Keep per-operation model for local SQLite.

## Batch API Pattern

```rust
pub async fn inject_concepts(
    &self,
    concepts: &[(String, HVec10240)]
) -> Result<()> {
    // Validate all inputs first
    // Batch insert to singularity
    // Batch save to persistence
    // Single transaction for DB
}
```

## Caching Pattern
- Prefer cached values stored as `Arc<[T]>` so cache hits are cheap (`Arc::clone`).
- Avoid keying caches via temporary `Vec` materializations; hash fixed-size words/arrays directly.

## Performance Targets

- Batch similarity: 10k ops/ms
- Connection pool: <1ms acquire time
- Cache hit rate: >80% for repeated access patterns
- Reservoir step: maintain <100μs @ 50k

## LOC Constraint
All files must remain ≤ 500 lines. Refactor to new modules if needed.
