# ADR-0004: Sparse Reservoir Matrix

## Status

Accepted (backfilled 2026-05-01)

## Context

The Echo State Network reservoir requires a weight matrix. For 50,000 nodes:
- Dense matrix: 50,000 x 50,000 = 2.5 billion entries
- Memory: ~10GB for f32 weights
- Step computation: O(n^2) per iteration

This is infeasible for in-memory operation.

## Decision

Use **CSR sparse matrix** with fixed degree k=64.

**Rationale:**
- Memory: O(n*k) instead of O(n^2)
- 50k nodes: ~25MB instead of ~10GB
- Step: O(n*k) instead of O(n^2)
- ESN literature shows sparse connectivity is sufficient
- Spectral radius control via weight scaling

## Consequences

### Positive
- Massive memory reduction (400x)
- Fast sparse matrix operations
- Cache-local traversal (fixed degree)
- Enables 50k node reservoir in <100MB

### Negative
- Fixed degree limits topology flexibility
- CSR format overhead vs dense for small n
- Weight initialization more complex

## Implementation

- Module: `src/reservoir.rs`
- Storage: Compressed Sparse Row (CSR) format
- Degree: k=64 (fixed per-node connections)
- Spectral radius: configurable (default 0.9)

## Sources

- ACTIONS.md lines 272-284 (sparse_reservoir_matrix action)
- MEMORY.md: "50,000-node sparse reservoir for temporal dynamics"
- Git: sparse matrix implementation commits