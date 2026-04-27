# Handoff: Group C -> Group D (Wave 20 Performance)

## Tasks
- IQ-16 add AVX2/NEON SIMD paths (gated + fallback)
- IQ-04 evaluate bincode->postcard migration impact

## Assumptions Passed
- `wasm32` remains on safe fallback path (no SIMD-specific code required).
- Performance claims must include benchmark deltas and correctness parity checks.

## Required Tests for D
- Feature-gated SIMD correctness parity vs scalar path
- Serialization compatibility tests for chosen migration strategy
- Bench threshold checks documented before CI gate
