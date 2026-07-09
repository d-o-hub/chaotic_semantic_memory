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

## Default/New Pattern (DeepSource BUG_RISK Prevention)

Construct directly in `Default::default()`, have `new()` delegate to `default()`.
This avoids DeepSource "Found call returning Self in default()" BUG_RISK violations.

```rust
// ✅ CORRECT: default() constructs directly; new() delegates
impl Default for MyBuilder {
    fn default() -> Self {
        Self { field: Default::default() }
    }
}
impl MyBuilder {
    pub fn new() -> Self { Self::default() }
}

// ❌ WRONG: triggers DeepSource BUG_RISK
impl Default for MyBuilder {
    fn default() -> Self { Self::new() }
}
```

## Map/Unwrap Pattern (DeepSource ANTI_PATTERN Prevention)

Prefer `.is_some_and()` / `.map_or()` / `.map_or_else()` over `.map().unwrap_or()`.
Enforced by `clippy::map_unwrap_or` (promoted to `warn` in `Cargo.toml`).

```rust
// ✅ CORRECT
concepts.get(id).is_some_and(|c| filter.matches(&c.metadata))
value.map_or_else(|| default(), |s| s.to_string())

// ❌ WRONG: triggers DeepSource ANTI_PATTERN + clippy::map_unwrap_or
concepts.get(id).map(|c| filter.matches(&c.metadata)).unwrap_or(false)
value.map(|s| s.to_string()).unwrap_or_else(|| default())
```

## Lint Policy: unwrap/expect/panic (Rust 2024+ Best Practice)

**Library code**: `unwrap_used`, `expect_used`, and `panic` are set to `warn` in
`[workspace.lints.clippy]` (Cargo.toml). CI runs `clippy -- -D warnings`, promoting
these to hard errors.

**Test code**: `.clippy.toml` sets `allow-unwrap-in-tests = true`,
`allow-expect-in-tests = true`, `allow-panic-in-tests = true`. This automatically
exempts `#[cfg(test)]` modules and `#[test]` functions — no per-file `#![allow]`
annotations needed.

**Justified production allows**: Use `#[allow(clippy::expect_used)]` with a comment
explaining why the operation cannot fail or why a panic is acceptable:

```rust
// Lock poisoning is unrecoverable — program state is corrupted
#[allow(clippy::expect_used)]
let cache = self.norm_cache.read().expect("lock poisoned");

// Prometheus metric construction is infallible with static params
#[allow(clippy::expect_used)]
let counter = IntCounterVec::new(opts, &["result"]).expect("counter");
```

**Never** use bare `unwrap()` in library code without an `#[allow]` + justification.
Prefer `?` operator, `.ok_or()`, `.map_err()`, or pattern matching.
