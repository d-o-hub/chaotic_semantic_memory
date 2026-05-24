# ADR-0068: HNSW ANN Index for >200k Scale

## Status

Proposed (2026-04-30)

## Context and Problem Statement

Per ADR-0056 the probe path is brute-force linear scan with Rayon + integer Hamming. Verified throughput at 50k concepts: 3.7 ms (well under 10 ms target). Extrapolating linearly:

| Concept count | Predicted probe time |
|---|---|
| 50k | 3.7 ms ✅ |
| 200k | ~15 ms ⚠ |
| 1M | ~75 ms ❌ |
| 10M | ~750 ms ❌ |

The trigger threshold (`probe_scale_trigger_exceeded` = true in GOAP_STATE) has been hit conceptually. We need an Approximate Nearest Neighbor (ANN) index to keep latency sub-10ms beyond 200k concepts.

## Decision Drivers

- Recall@10 ≥ 0.95 vs exact (Hamming integer) baseline
- p50 probe latency ≤ 5 ms at 1M concepts
- Memory overhead ≤ 2× the f32 hypervector storage
- Must support insert / delete / update online (no rebuild step)
- LOC budget ≤ 500/file
- Should be **opt-in** behind a feature flag — small deployments stay zero-overhead

## Considered Options

1. **HNSW** (Hierarchical Navigable Small Worlds) — `hnsw_rs` crate, mature, online insert
2. **IVF** (Inverted File Index) — needs k-means training, periodic rebuild, awkward online
3. **LSH** (Locality-Sensitive Hashing) — natural fit for Hamming distance, no training
4. **ScaNN-style** product quantization — high recall, heavy preprocessing, no Rust crate

## Decision Outcome

Chosen: **Option 1 (HNSW)** primary with **Option 3 (LSH)** as a feature-flag alternative.

Rationale:
- HNSW gives best recall/latency trade-off for cosine and Hamming
- `hnsw_rs` is pure Rust, no FFI, supports custom distance functions
- LSH alternative for Hamming-only deployments where memory matters more than recall

## Implementation

### Module layout

```
src/index/
  mod.rs          # IndexBackend enum + trait
  brute_force.rs  # current path, refactored behind trait
  hnsw.rs         # HNSW backend (opt-in)
  lsh.rs          # LSH Hamming backend (opt-in)
```

Each file ≤ 400 LOC.

### Trait

```rust
pub trait AnnIndex: Send + Sync {
    fn insert(&mut self, id: ConceptId, vec: &HVec10240) -> Result<()>;
    fn delete(&mut self, id: &ConceptId) -> Result<()>;
    fn search(&self, query: &HVec10240, top_k: usize) -> Result<Vec<(ConceptId, f32)>>;
    fn rebuild(&mut self) -> Result<()>;
    fn stats(&self) -> IndexStats;
}
```

### Singularity wiring

```rust
pub struct Singularity {
    concepts: HashMap<String, Concept>,
    index: Box<dyn AnnIndex>,  // BruteForce default; Hnsw / Lsh opt-in
    ...
}
```

### Configuration

```rust
pub enum IndexBackend {
    BruteForce,                    // default, exact
    Hnsw { m: usize, ef_construction: usize, ef_search: usize },
    Lsh { num_tables: usize, hash_bits: usize },
}

framework_builder.with_index_backend(IndexBackend::Hnsw {
    m: 16, ef_construction: 200, ef_search: 50
})
```

### Persistence

- HNSW graph serialized alongside concepts in libSQL (new table `hnsw_graph`)
- Migration `007_add_hnsw_graph.sql`
- On load: detect serialized graph, deserialize; otherwise rebuild from concepts

### Cargo features

```toml
[features]
ann-hnsw = ["dep:hnsw_rs"]
ann-lsh = []   # pure Rust, no extra deps
```

### Benchmarks (new bench targets)

- `bench_probe_brute_force_{50k,200k,1M}`
- `bench_probe_hnsw_{50k,200k,1M}`
- `bench_probe_lsh_{50k,200k,1M}`
- Recall comparison report (HNSW/LSH vs brute force ground truth)

## Pros and Cons

### Pros
- Removes scale ceiling — production-grade beyond 1M concepts
- Backward compatible (default backend unchanged)
- HNSW is widely understood, mature

### Cons
- New optional dep `hnsw_rs` (~150 KB compiled)
- Persistence migration required
- Recall is approximate (~0.95-0.98 typical)
- ef_search tuning needed per workload

## Acceptance Criteria

- [ ] All three backends implement `AnnIndex` trait
- [ ] BruteForce remains default; existing tests pass unchanged
- [ ] HNSW recall@10 ≥ 0.95 vs brute force on 1M synthetic dataset
- [ ] HNSW p50 probe ≤ 5 ms at 1M concepts
- [ ] Persistence roundtrip preserves index
- [ ] Migration works on existing v0.3 databases
- [ ] All `src/index/*.rs` files ≤ 400 LOC
- [ ] Benchmark report committed to `benchmarks/results/ann/`
