---
name: benchmarking-perf
description: "Run and analyze criterion benchmarks for performance-sensitive changes. Use when optimizing hot paths, validating perf targets, or comparing baselines."
---

# Benchmarking & Performance

## Performance Targets

| Benchmark | Target | Current (v0.3.0) |
|---|---|---|
| `reservoir_step_50k` | < 100μs | ~102μs (close, parallel input projection) |
| `cosine_similarity` | < 1μs | ~0.23μs ✅ |
| `batch_similarity_1000` | < 500μs | ~322μs ✅ |
| `hvec_random` | < 5μs | ~0.82μs ✅ |
| `hvec_bind` | < 1μs | ~0.08μs ✅ |
| `bm25_search_1000` | < 5ms | ~2.41ms ✅ |

## Workflow

### 1. Save Baseline Before Changes
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo bench --bench benchmark -- --save-baseline before 2>&1 | grep -E "(^test |time:|Benchmark|bench_|[0-9]+\.[0-9]+ µs)" | tail -30
```

### 2. Make Changes

### 3. Compare Against Baseline
```bash
export CARGO_TERM_PROGRESS_WHEN=never
cargo bench --bench benchmark -- --baseline before 2>&1 | grep -E "(time:|Benchmark|bench_|change:)" | tail -30
```

### 4. Interpret Results
- Look for `time: [lower upper]` in criterion output.
- Green = faster, Red = slower.
- Changes > 5% in hot paths require investigation.

## Adding a New Benchmark
Edit `benches/benchmark.rs`. Follow existing patterns:
```rust
fn bench_my_operation(c: &mut Criterion) {
    // Setup outside the closure
    let data = prepare_data();

    c.bench_function("my_operation", |b| {
        b.iter(|| {
            // Only the measured code here
            my_operation(black_box(&data))
        })
    });
}
```
Add to `criterion_group!` at the bottom.

## Gotchas
- Never `--baseline` without first `--save-baseline` with the same name.
- Don't capture mutable state by reference in criterion closures.
- Use `black_box()` on inputs AND outputs to prevent dead-code elimination.
- Reservoir benchmarks use `new_seeded(..., 42)` for reproducibility.
