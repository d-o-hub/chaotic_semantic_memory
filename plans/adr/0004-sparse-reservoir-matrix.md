# [ADR-0004] Sparse Reservoir Weight Matrix

## Status
Proposed

## Context and Problem Statement
The current `Reservoir` implementation uses dense `ndarray::Array2<f32>` for the reservoir weight matrix `w_res`. With the default size of 50,000 nodes:
- Dense matrix: 50,000 × 50,000 × 4 bytes = **~10 GB RAM** just for `w_res`
- Initialization is O(n²) even though only ~10% of entries are non-zero
- Matrix-vector multiplication in `step()` is O(n²) instead of O(n·k)
- This makes the default configuration physically impossible to run

The echo state network literature consistently uses sparse connectivity (typically 1–10% density with fixed degree per neuron).

## Decision Drivers
* Default reservoir must be runnable on standard hardware (16–64 GB RAM)
* Reservoir step must complete in < 100μs at target size
* Initialization must be O(n·k) not O(n²)
* Must maintain edge-of-chaos dynamics (spectral radius control)
* Must stay within 500 LOC file limit for reservoir.rs

## Considered Options
1. **Custom CSR (Compressed Sparse Row) with fixed out-degree `k`**
2. **Use `sprs` crate for sparse matrices**
3. **Keep dense but reduce default size drastically (e.g., 2048)**

## Decision Outcome
Chosen option: **Custom CSR with fixed out-degree `k`**, because:
- No new dependency needed (small struct: `row_offsets`, `col_idx`, `weights`)
- Fixed degree `k` (default 64) gives predictable memory: O(n·k)
- Init cost: O(n·k) — generate `k` random neighbors per neuron
- Step cost: O(n·k) for sparse matvec — achievable in < 100μs
- Simple power iteration still works for spectral radius estimation
- Stays well under 500 LOC

### Memory comparison at n=50,000, k=64
| Representation | Memory for w_res | Init time |
|---|---|---|
| Dense Array2 | ~10 GB | ~minutes |
| CSR k=64 | ~25 MB | ~ms |

### Positive Consequences
* Default config (50k nodes) becomes runnable on commodity hardware
* Step latency drops from O(n²) to O(n·k) — target < 100μs achievable
* Init goes from seconds/minutes to milliseconds
* Memory footprint drops ~400x

### Negative Consequences
* Spectral radius estimation via power iteration may converge slower on sparse matrices
* Fixed degree limits some reservoir topologies (mitigated: k is configurable)
* Cannot trivially use ndarray BLAS routines (but sparse matvec is simple to implement)

## Implementation Sketch
```rust
struct SparseMatrix {
    n: usize,
    k: usize,  // fixed out-degree
    row_offsets: Vec<usize>,  // length n+1
    col_idx: Vec<u32>,        // length n*k
    weights: Vec<f32>,        // length n*k
}

impl SparseMatrix {
    fn matvec(&self, x: &[f32], out: &mut [f32]) {
        for i in 0..self.n {
            let start = self.row_offsets[i];
            let end = self.row_offsets[i + 1];
            let mut sum = 0.0f32;
            for idx in start..end {
                sum += self.weights[idx] * x[self.col_idx[idx] as usize];
            }
            out[i] = sum;
        }
    }
}
```

## Pros and Cons of the Options

### Custom CSR with fixed degree
* Good: Zero new dependencies
* Good: Predictable O(n·k) memory and compute
* Good: Simple implementation (~60 LOC)
* Bad: No BLAS acceleration (acceptable for sparse)

### sprs crate
* Good: Mature sparse matrix library
* Good: Supports CSR/CSC natively
* Bad: New dependency to maintain
* Bad: May be overengineered for fixed-degree use case

### Dense with reduced size
* Good: No code changes needed
* Bad: Limits reservoir expressiveness
* Bad: Does not solve the fundamental scaling problem
* Bad: 2048 nodes may be too small for complex temporal dynamics
