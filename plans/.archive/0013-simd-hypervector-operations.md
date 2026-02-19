# [ADR-0013] SIMD-Accelerated Hypervector Operations

## Status
Accepted

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
Chosen option: **target-specific SIMD intrinsics on x86/x86_64 with scalar fallback**

### Implementation Strategy
- Use `std::arch` intrinsics (`_mm_xor_si128` load/xor/store path) for native x86/x86_64
- Accelerate `bind` and `cosine_similarity` hot loops without changing public API
- Keep scalar fallback for non-x86 and wasm targets
- Preserve existing serialized representation (`[u128; 80]` / 1280-byte format)

### Positive Consequences
- Native hot-path throughput improves without requiring nightly features
- No new dependencies added
- Public data format and behavior remain backward compatible

### Negative Consequences
- Intrinsics path currently targets x86/x86_64 only
- Slightly more unsafe code with architecture guards
- Diminishing returns for small batches

## Links
- [Rust Portable SIMD](https://doc.rust-lang.org/std/simd/index.html)
- [Target Feature Documentation](https://rust-lang.github.io/packed_simd/perf-guide/target-feature.html)
