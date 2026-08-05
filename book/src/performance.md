# Performance

## Benchmarks

Run benchmarks:

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
cargo bench --bench binary_benchmark               # BHVec10240::hamming direct dispatch
cargo bench --bench graph_candidates_benchmark     # graph candidate retrieval path
```

CI runs `benchmark-graph-candidates` on `benches/**`, `csm-memory`, and workflow
changes, enforcing a documented regression ceiling (~600 µs; measured ~259 µs on
GitHub runners).

## Targets

| Metric | Target | Actual |
|--------|--------|--------|
| `reservoir_step_50k` | <100μs | ~76μs |
| `BHVec10240::hamming` (direct dispatch, PR #597) | — | ~37.7 ns idle / ~54.5 ns loaded (~2.6–2.75× vs `to_hvec()` conversion path) |
| Graph candidate retrieval (PR #598) | — | ~218 µs on 500-node graph (~8% faster than String-clone baseline) |
| `turso_roundtrip` | <20ms | Passing |
| `10m_concepts_memory` | <12MB | Passing |
| `wasm_binary_size` | <500KB | ~438KB |

## Tuning Guide

### Small Workloads (<10k concepts)

```rust
let framework = ChaoticSemanticFramework::builder()
    .without_persistence()
    .with_reservoir_size(10_240)
    .with_concept_cache_size(128)
    .build()
    .await?;
```

### Medium Workloads (10k-1M concepts)

```rust
let framework = ChaoticSemanticFramework::builder()
    .with_local_db("csm_memory.db")
    .with_max_concepts(1_000_000)
    .with_concept_cache_size(1_000)
    .build()
    .await?;
```

### Large Workloads (>1M concepts)

```rust
let framework = ChaoticSemanticFramework::builder()
    .with_remote_db(turso_url, auth_token)
    .with_connection_pool_size(20)
    .with_max_concepts(10_000_000)
    .with_max_probe_top_k(1_000)
    .with_concept_cache_size(10_000)
    .build()
    .await?;
```

## Memory Footprint

| Components | Per-Unit | 1M Concepts |
|------------|----------|-------------|
| Concept ID | ~32 bytes | 32 MB |
| HVec10240 | 1,280 bytes | 1.28 GB |
| Associations | ~24 bytes/edge | Varies |

Tips:
- Set `max_concepts` to enforce memory ceiling
- Use `max_associations_per_concept` to limit edges
- Cache hit rate improves with `concept_cache_size`

## Optimization Techniques

### SIMD Hypervector Operations

Automatic via std::simd on x86-64:

```rust
// These use SIMD when available
HVec10240::bundle(&vectors);
HVec10240::cosine_similarity(&a, &b);
```

`BHVec10240::hamming` dispatches directly over the packed `[u64; 160]` words —
AVX2 popcount kernels on x86_64, NEON on aarch64, and an unrolled scalar
`u64::count_ones()` fallback (including wasm32) — skipping the two `to_hvec()`
layout conversions the previous implementation performed per distance call
(PR #597, ~2.6–2.75× faster same-machine). The NEON kernels are executed in CI
on a native arm64 runner (PR #599).

### Graph Candidate Retrieval

`generate_graph_candidates` (the association-graph BFS used when
`enable_graph_candidates` is set) borrows `&str` from the seed results and the
association map instead of cloning every candidate `String` — eliminating the
per-candidate heap allocations on the expansion path (PR #598, ~8% faster
end-to-end on a 500-node association graph).

### Parallel Similarity Search

Automatic via Rayon:

```rust
// Uses all cores for similarity computation
framework.probe(vector, 100).await?;
```

### Batch Operations

Prefer batch over individual:

```rust
// Slower: N round trips
for (id, vec) in concepts {
    framework.inject_concept(id, vec).await?;
}

// Faster: 1 round trip
framework.inject_concepts(&concepts).await?;
```

### Connection Pooling

For remote databases:

```rust
.with_connection_pool_size(20) // More connections for concurrent access
```
