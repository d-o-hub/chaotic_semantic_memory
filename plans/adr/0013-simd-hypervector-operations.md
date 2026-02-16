# [ADR-0013] SIMD-Accelerated Hypervector Operations

## Status
Proposed

## Context and Problem Statement
Current hypervector operations (bundle, bind, cosine_similarity) use scalar loops over `[u128; 80]`. While already fast, batch operations could benefit from SIMD parallelism for higher throughput in data-intensive workloads.

## Decision Drivers
- Batch similarity/search operations need higher throughput
- Modern CPUs support 128-bit and 256-bit SIMD operations
- Memory bandwidth is often the bottleneck, not compute
- Must maintain backward compatibility and correctness

## Considered Options
1. **Scalar (current)** - Portable, sufficient for most use cases
2. **std::simd (portable_simd)** - Rust standard library SIMD, portable across architectures
3. **target-specific intrinsics** - AVX2, NEON, etc. Maximum performance but requires multiple implementations
4. **External crates (packed_simd)** - More mature but additional dependency

## Decision Outcome
Chosen option: **std::simd** from `std::simd` (nightly) or `portable_simd` crate

### Implementation Strategy
- Use `u64x4` or `u128x2` for vectorized operations
- Bundle: Parallel popcount across lanes
- Cosine similarity: Parallel equality comparison
- Maintain scalar fallback for non-SIMD targets

### Positive Consequences
- 2-4x throughput improvement for batch operations
- Single implementation works on x86_64, aarch64, wasm32
- Future-proof as portable_simd stabilizes

### Negative Consequences
- Adds nightly Rust requirement (until portable_simd stabilizes)
- Slightly more complex code
- Diminishing returns for small batches

## Links
- [Rust Portable SIMD](https://doc.rust-lang.org/std/simd/index.html)
- [Target Feature Documentation](https://rust-lang.github.io/packed_simd/perf-guide/target-feature.html)
