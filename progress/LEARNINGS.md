# LEARNINGS - Chaotic Semantic Memory

## Core Patterns & Best Practices

### Security Patterns
- **Path Hijacking Mitigation (CWE-426)**: Always resolve system executables (e.g., `git`) to absolute paths. Filter the `PATH` environment variable to strictly exclude relative entries like `.` or empty paths before use in `Command::new()`.
- **DoS Prevention**: Enforce strict upper bounds on all public API parameters. For graph traversals, use `MAX_TRAVERSAL_DEPTH = 32` and `MAX_TRAVERSAL_RESULTS = 10,000`. Batch operations should be limited to `max_batch_size` (default 1000).

### Optimization Patterns
- **Instruction-Level Parallelism (ILP)**: Use independent accumulators (e.g., 4) in hot loops (popcount, dot products) to break serial dependency chains. This often outperforms SIMD due to avoiding STLF stalls.
- **Branchless Logic**: Use branchless bitmask construction (`w |= (condition as u128) << j`) to minimize branch misprediction penalties in tight loops.
- **Zero-Allocation Interning**: Use `Arc<str>` interning for terms (e.g., BM25) to share memory. Use a `get_mut`/`get_key_value` double-lookup pattern to update frequencies without new allocations for existing terms.
- **Algorithmic Parallelism**: Parallelize $O(N)$ scans using Rayon's `par_iter()`. For bitmask-based retrieval, this reduces latency from $O(N)$ to $O(N/P)$.
- **Bitmask Filtering**: For power-of-2 bucket counts, replace modulo (`%`) with bitmask AND (`&`) for significant speedups.

### Performance Baselines (x86_64)
- `HVec10240::hamming_distance`: ~219 ns (unrolled GPR loop).
- `HVec10240::cosine_similarity`: ~238 ns (unified GPR path).
- Reservoir Step (50k nodes): ~136 µs (4 accumulators).
- BM25 Search (10k docs): ~406 µs (hoisted constants).

## Technical Insights by Module

### Reservoir & ESN
- **Partial Updates**: Partitioned updates (rotating node subsets) must preserve momentum. Reverting to a partial `prev_state` update loop is critical for maintaining correct inertial state in partitioned ESNs.
- **Memory Locality**: For large sparse reservoirs, CSR-like contiguous arrays and neighborhood connectivity dominate throughput by reducing random memory access.

### Retrieval & Similarity
- **Unified Similarity**: Deriving Cosine Similarity from Hamming Distance (`1.0 - (dist / 5120.0)`) for bipolar hypervectors is optimal across all architectures, saving a `NOT` instruction and unifying SIMD/scalar paths.
- **Top-K Selection**: Use `select_nth_unstable_by` for $O(N)$ partial sorting instead of $O(N \log N)$ full sort.

### Persistence & Environment
- **Namespace Isolation**: Use prefixed table names (`csm_concepts`, etc.) for shared databases. Legacy names in migration scripts are intentional.
- **WASM Compliance**: Always gate `rayon` and I/O paths with `#[cfg(not(target_arch = "wasm32"))]`. Use `wasm-bindgen-futures` for async WASM exports.
- **Sandbox Limits**: Intermittent network timeouts during `cargo fetch` are common. Verify core logic using standalone `rustc` scripts if dependencies are minimal.

## What to Avoid
- **Unqualified Commands**: Never use bare command names like `Command::new("git")`.
- **Insecure PATH**: Never trust `PATH` to contain only absolute, trusted directories.
- **Floating Point Comparison**: Never use `partial_cmp().unwrap()` on floats; NaN will panic. Use `f32::total_cmp()`.
- **Schema Drift**: Always update all persistence surfaces (single, batch, export, WASM) when adding concept fields.
- **Dense Matrices**: Avoid dense matrices for reservoirs > 2000 nodes; use CSR.
- **Archived Deps**: Never use archived GitHub repositories as dependencies.
