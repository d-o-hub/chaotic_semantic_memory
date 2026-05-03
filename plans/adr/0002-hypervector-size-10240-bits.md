# ADR-0002: Hypervector Size (10240 bits)

## Status

Accepted (backfilled 2026-05-01)

## Context

Hypervector dimensionality is a critical design decision affecting:
- Memory footprint
- Similarity discrimination
- Bundle operation quality
- SIMD optimization potential

Common sizes in HDC research: 512, 1024, 4096, 8192, 10000, 16384

## Decision

Use **10240 bits** (10,240-dimensional binary hypervectors).

**Rationale:**
- Standard size in HDC literature (close to 10,000)
- Sufficient for semantic discrimination in AI memory
- Fits in 80 x 128-bit words (u128 array)
- SIMD-friendly (16-byte aligned chunks)
- Memory-efficient: 1280 bytes per vector

## Consequences

### Positive
- Good semantic capacity for concept storage
- Efficient bit operations with u128 SIMD
- Reasonable memory footprint (1.28KB per concept)
- Supports bundle, bind, permute operations

### Negative
- Larger than minimal (512) but smaller than maximum (16384)
- Fixed size limits configurability (see ADR-0060 for future)

## Implementation

- Type: `HVec10240` in `src/hyperdim.rs`
- Storage: `[u128; 80]` array (80 words x 128 bits = 10240)
- Operations: bundle (XOR popcount), bind (XOR), permute (shift)

## Sources

- MEMORY.md: "10,240-bit hypervectors for semantic representation"
- src/hyperdim.rs: HVec10240 struct definition
- ACTIONS.md: bundle_associativity tests