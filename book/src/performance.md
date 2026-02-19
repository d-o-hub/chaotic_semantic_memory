# Performance

## Benchmarks

Run benchmarks:

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```

## Targets

| Metric | Target | Actual |
|--------|--------|--------|
| `reservoir_step_50k` | <100μs | ~76μs |
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
    .with_local_db("memory.db")
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
