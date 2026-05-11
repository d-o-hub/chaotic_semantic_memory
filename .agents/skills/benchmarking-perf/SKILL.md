---
name: benchmarking-perf
description: "Run and analyze criterion benchmarks for performance-sensitive changes. Use when optimizing hot paths, validating perf targets, or comparing baselines. Covers SIMD, connection pooling, batch APIs, and caching."
---

# Benchmarking & Performance

## Performance Targets

| Benchmark | Target | Current (v0.3.0) |
|---|---|---|
| `reservoir_step_50k` | < 100μs | ~57μs ✅ |
| `cosine_similarity` | < 1μs | ~0.15μs ✅ |
| `batch_similarity_1000` | < 500μs | ~280μs ✅ |
| `hvec_random` | < 5μs | ~0.49μs ✅ |
| `hvec_bind` | < 1μs | ~0.07μs ✅ |
| `bm25_search_10000` | < 5ms | ~2.2ms ✅ |
| `singularity_probe_50000` | < 10ms | ~3.7ms ✅ |

## Scalability Results

| Concept Count | Probe Time |
|--------------|------------|
| 100 | ~1.3ms |
| 1,000 | ~1.6ms |
| 10,000 | ~1.6ms |
| 50,000 | ~3.7ms |

## Workflow

### 1. Save Baseline Before Changes
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo bench --bench benchmark -- --save-baseline before
```

### 2. Compare Against Baseline
```bash
cargo bench --bench benchmark -- --baseline before
```

### 3. Interpret Results
- Green = faster, Red = slower
- Changes > 5% in hot paths require investigation

## SIMD Optimization

```rust
#[cfg(feature = "simd")]
use std::simd::u128x2;

pub fn cosine_similarity_simd(&self, other: &Self) -> f32 {
    // Use u128x2 for parallel operations
    // Fall back to scalar for WASM/non-SIMD targets
}
```

Always provide scalar fallback for non-SIMD targets. Gate with feature flags.

## Connection Pooling

Use `deadpool` for async connection pooling, gated for remote Turso only.
Keep per-operation model for local SQLite.

## Batch API Pattern

```rust
pub async fn inject_concepts(&self, concepts: &[(String, HVec10240)]) -> Result<()> {
    // Validate all inputs first → Batch insert → Single transaction for DB
}
```

## Caching Pattern
- Prefer cached values as `Arc<[T]>` for cheap `Arc::clone` hits
- Avoid keying caches via temporary `Vec` materializations; hash fixed-size words/arrays directly
- Cache hit rate target: >80% for repeated access patterns

## Adding a New Benchmark
Edit `benches/benchmark.rs`. Follow existing patterns:
```rust
fn bench_my_operation(c: &mut Criterion) {
    let data = prepare_data();
    c.bench_function("my_operation", |b| {
        b.iter(|| my_operation(black_box(&data)))
    });
}
```
Add to `criterion_group!` at the bottom.

## Gotchas
- Never `--baseline` without first `--save-baseline` with the same name
- Don't capture mutable state by reference in criterion closures
- Use `black_box()` on inputs AND outputs to prevent dead-code elimination
- Reservoir benchmarks use `new_seeded(..., 42)` for reproducibility

## LOC Constraint
All files must remain ≤ 500 lines. Refactor to new modules if needed.
