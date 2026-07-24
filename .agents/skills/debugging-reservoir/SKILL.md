---
name: debugging-reservoir
description: "Debug and tune the echo state network reservoir. Use when diagnosing spectral radius issues, chaotic dynamics problems, sparse weight anomalies, or reservoir-to-hypervector projection failures."
---

# Debugging the Reservoir

## Architecture Overview
The reservoir is a sparse Echo State Network in `crates/csm-core-lib/src/`:
- `reservoir.rs`: core ESN with sparse adjacency-list weights
- `reservoir_chaotic.rs`: wrapper adding noise perturbation for edge-of-chaos dynamics
- `reservoir_sparse.rs`: sparse weight utilities
- `reservoir_inertial.rs`: inertial variant with momentum
- `reservoir_tests.rs`: test suite

## Sparse Weight Format
Weights are `Vec<Vec<(usize, f32)>>` — each row is a list of `(column_index, weight)`.
- Input weights (`w_in`): 32 connections per neuron
- Reservoir weights (`w_res`): 64 connections per neuron (or `min(64, size)`)
- Dot product via `dot_sparse_row(&weights[i], &values)`

## Spectral Radius Debugging
The spectral radius controls chaos level. Must be in `[0.9, 1.1]`.

### Symptoms of wrong radius
| Symptom | Likely Cause |
|---|---|
| State saturates to all ±1 | Radius too high (> 1.1) |
| State decays to all ~0 | Radius too low (< 0.9) |
| Non-deterministic test failures | Using `new()` instead of `new_seeded()` |
| Different results across runs | Unseeded RNG |

### How radius is estimated
Power iteration (16 steps) in `estimate_spectral_radius()`:
1. Start with uniform vector `v`
2. Repeat: `v = W * v`, then normalize
3. Compute Rayleigh quotient: `v^T W v / v^T v`

If radius is wrong after init, check:
- Are weights being scaled? (`scale = target / current`)
- Is `current_radius` returning 0? (degenerate weights)
- Is degree too low? (< 8 may give poor estimates)

## to_hypervector() Projection
Projects reservoir state → `HVec10240`. Requires `size >= 10240`.
- Divides state into 10240 chunks
- Each chunk sum > 0 → bit = 1

### Common errors
- `InvalidDimension { expected: 10240, actual: N }` — reservoir too small
- All-zero hypervector — reservoir state hasn't been activated (call `step()` first)

## Chaotic Reservoir Noise
`ChaoticReservoir` adds uniform noise `±chaos_strength` to each input element.
Uses a separate RNG seeded with `seed ^ 0xA5A5_5A5A_F0F0_0F0F` for independence.

## Diagnostic Checklist
1. Is `step()` being called before `to_hypervector()`?
2. Is the reservoir `reset()` between independent sequences?
3. Is `size >= 10240` for projection?
4. Is spectral radius in `[0.9, 1.1]` after construction?
5. Are tests using `new_seeded(..., 42)` for determinism?
