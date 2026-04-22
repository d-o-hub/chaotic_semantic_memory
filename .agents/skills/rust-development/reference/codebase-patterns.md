# Codebase Patterns

## WASM Rayon Gating
Every file that uses Rayon must have this pattern:

```rust
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
```

And at each call site:
```rust
#[cfg(not(target_arch = "wasm32"))]
let results: Vec<_> = items.par_iter().map(|x| compute(x)).collect();

#[cfg(target_arch = "wasm32")]
let results: Vec<_> = items.iter().map(|x| compute(x)).collect();
```

Currently gated in: `hyperdim.rs`, `reservoir.rs`, `singularity.rs`, `bm25.rs`.

## Sparse Reservoir Weights
Reservoir uses `Vec<Vec<(usize, f32)>>` adjacency lists, NOT dense `Array2`:
- Fixed input degree: 32 connections per neuron
- Fixed reservoir degree: 64 connections per neuron
- Sparse dot product via `dot_sparse_row()`
- Spectral radius estimated via 16-iteration power method

## Persistence Per-Operation Connections
`Persistence` stores `Arc<Database>` and creates a fresh `Connection` per operation:
```rust
fn connect(&self) -> Result<Connection> {
    self.db.connect().map_err(|e| MemoryError::Database(...))
}
```
Do NOT use `Arc<RwLock<Connection>>` — it has Send/Sync issues under tokio.

## Batch Persistence
Use `save_concepts()` / `save_associations()` with `execute_batch()` for bulk operations.
Wrap multi-statement operations in `BEGIN; ... COMMIT;`.

## Similarity Search
`find_similar()` uses Rayon `par_iter()` + `select_nth_unstable_by()` for partial top-k.
Always use `f32::total_cmp()` instead of `partial_cmp().unwrap()` — NaN safety.

## Query Result Cache
`Singularity` caches similarity results keyed by `(top_k, query.data)` (hashing words directly to avoid `to_bytes()` allocations).
Use `find_similar_cached()` when you want cache-hit reuse via `Arc<[(String, f32)]>`.

## Error Handling
Use `MemoryError` variants, never `anyhow` in library code:
- `Database(String)` — libsql errors
- `InvalidDimension { expected, actual }` — size mismatches
- `Reservoir(String)` — ESN errors
- `Persistence(String)` — concept not found, etc.

## Seeded RNG for Reproducibility
Reservoir uses `StdRng::seed_from_u64(seed)`:
```rust
pub fn new_seeded(input_size: usize, size: usize, seed: u64) -> Result<Self>
```
Tests should use `new_seeded(..., 42)` for determinism.
