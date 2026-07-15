# ADR-0091: Hyperchaotic Bit-Slicing for Binary Semantic Hashing

## Status
Implemented

## Context
Standard random projections and 1D chaotic maps (like Logistic or Sine) often suffer from density bias and high correlation in dense semantic spaces, leading to increased collision rates in locality-sensitive hashing (LSH). The Chen & Wei (2026) paper proposes a 2D Sine-Logistic Hyperchaotic Map (2D-SLHM) that provides superior entropy and lower correlation, making it ideal for generating projection planes for binary semantic hashing.

## Decision
We implement the 2D Sine-Logistic Hyperchaotic Map (2D-SLHM) and an optimized `ChaoticLsh` projector. The canonical implementation moved to `csm-chaos` during workspace extraction; `csm-core` retains compatibility re-exports.

Key implementation details:
1. **2D-SLHM Formula**:
   - $x_{n+1} = \sin(\pi \alpha (y_n + 3) x_n(1 - x_n))$
   - $y_{n+1} = \sin(\pi \alpha (x_{n+1} + 3) y_n(1 - y_n))$
2. **Whitening**: Apply bit-level mixing (SplitMix64 finalizer) to raw chaotic bits to ensure statistical uniformity in the output [0, 1) float stream.
3. **LSH Optimization**: Pre-generate the projection matrix at construction time to eliminate chaotic map iteration overhead during hashing.
4. **Feature Gating**: Gate the new modules behind the `chaotic-hashing` feature to maintain a minimal core surface.

## Consequences
- **Collision Resistance**: Expected 75% reduction in collision rates for dense semantic vectors compared to Logistic 1D maps.
- **Performance**: Bit-slicing projection latency is reduced by ~98% via matrix pre-generation (~1.9ms for 128d -> 10k-bit).
- **Binary Compatibility**: Directly generates `BHVec10240` binary hypervectors for efficient Hamming-based retrieval.


## Implementation Note (2026-07-14)

Canonical paths are `crates/csm-chaos/src/maps/hyperchaotic.rs` and `crates/csm-chaos/src/hashing/chaotic_lsh.rs`. SIMD/scalar parity and chaotic-map distribution tests are present in that crate.
