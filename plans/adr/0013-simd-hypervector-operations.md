# ADR-0013: SIMD Hypervector Operations

## Status

Accepted (backfilled 2026-05-01)

## Context

Hypervector operations are performance-critical:
- Bundle: popcount across 80 u128 words
- Cosine similarity: equality count
- Bind: XOR operation

Original: scalar loops, O(80) per operation

## Decision

Use **std::simd for parallel operations** across lanes.

**Rationale:**
- u128x4 SIMD lanes for 4 words at once
- Bundle: parallel popcount across lanes
- Cosine similarity: SIMD equality count
- Target: 2-4x throughput improvement

## Consequences

### Positive
- Significant speedup for batch operations
- Utilizes CPU SIMD capabilities
- Portable across architectures (x86, ARM)
- Safe std::simd API

### Negative
- SIMD intrinsics require unsafe blocks on some platforms
- NEON intrinsics need explicit unsafe for aarch64
- WASM requires scalar fallback

## Implementation

- Module: `src/hyperdim.rs`
- SIMD: std::simd u128x4 lanes
- Platforms: x86 (SSE2+), aarch64 (NEON)
- WASM fallback: scalar loops

## Sources

- ACTIONS.md lines 550-565 (implement_simd_hypervector_ops action)
- src/hyperdim.rs: SIMD blocks with SAFETY comments
- Git: fix(simd): add explicit unsafe block for NEON intrinsics