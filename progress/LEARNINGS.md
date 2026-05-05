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

## Autonomous PR Repair (Root Cause Analysis - May 2026)
- **Versioning Logic**: Native framework versioning (v1, v2) is triggered by updating a concept with a stable ID. Manually appending suffixes like `:v1` in the benchmark generator creates separate concepts and breaks lineage evaluation.
- **Merge Conflict Strategy**: When rebasing or merging, prioritize preserving functional logic from both sides. Use `git checkout origin/main -- <file>` to restore files lost during complex merges/rebases.
- **Optimization Strategy**: Gating parallelization (e.g., Rayon) with a minimum workload threshold (N >= 32) prevents task scheduling overhead from dominating small operations, yielding order-of-magnitude gains in hot paths like hypervector bundling.

## Open-PR Triage (2026-05-04)
- **`benchmarks/` is NOT a workspace member**: it's a separate crate with its own `Cargo.toml`/`Cargo.lock`. Root-level `cargo check --workspace --all-targets` does **NOT** compile it. Validation must explicitly run `( cd benchmarks && cargo check )` or CI's `benchmark-small` job will surface errors (e.g., missing imports, duplicate fns) that never appeared locally. Encoded into `scripts/validate.sh` and `scripts/pre-commit.sh`.
- **Squash-merging across diverged branches yields broken trees**: `git merge --squash` on a long-lived branch can produce a worktree with unresolved conflict semantics (duplicate `query_association`, missing `NamedTempFile`/`GraphRagConfig` imports, repeated struct fields) even when conflict markers are removed. After squash-merge, run **all** validation gates including the standalone `benchmarks/` check before pushing. If broken, restore the affected file from main with `git checkout main -- path/to/file.rs` and reapply only the targeted change.
- **Triage stale PRs by diff direction, not by feedback count**: If a PR's branch would *delete* lines that are valuable on `main` (e.g., #169 wanted to remove 7044 LOC including #167 GraphRAG tests; #159 would delete `singularity_search.rs` from #173), close it. Re-rebasing such PRs through cascading conflicts is more expensive than re-filing the residual feedback as fresh issues against current `main`.
- **Auto-merge gotcha**: `gh pr merge --auto` returns silently and does not block. Use `--admin` (when authorized) or `gh pr checks --watch` + explicit `gh pr merge` for deterministic completion.

## Namespace Isolation PR #178 (2026-05-05)
- **Transactional SQL**: `conn.execute()` with multi-statement SQL (`BEGIN; DELETE...; COMMIT;`) does not guarantee atomicity in libsql. Use explicit `conn.execute("BEGIN", ())` + individual parameterized DELETE statements + `conn.execute("COMMIT", ())` with rollback on error. This also prevents SQL injection from string interpolation.
- **Namespace in restore**: Schema v8 adds `namespace` to all table PKs. Backup/restore INSERT statements must include `namespace` in both column list and SELECT projection, or multi-namespace data corrupts into `_default` (PK collisions, data loss).
- **Missing namespace = empty, not panic**: `find_similar_filtered` and other retrieval methods should return empty results for a missing namespace, not `expect()`/`panic()`. Fresh frameworks or callers with typos in namespace names should degrade gracefully.
- **Persist before in-memory drop**: `delete_namespace` must clear the DB first. If DB call fails, the in-memory state stays intact (safe to retry). Reversing the order (drop in-memory first) leaves orphaned DB rows on failure.
- **Cyclomatic complexity reduction**: Extract early-return helper methods (`try_cache_hit`, `try_ann_search`, `generate_candidates`) from complex functions to reduce cyclomatic complexity below 15 (DeepSource threshold).
- **Merge conflict pattern (namespace + observability)**: When rebasing a feature branch (namespace params) onto main (observability metrics), both modify the same method bodies. Resolution requires combining both: namespace params on Singularity/Persistence calls AND observability timing around persistence calls.
- **Clap arg duplication**: Having `--namespace` on both `CliArgs` (global) and per-subcommand `Args` causes clap conflicts. Keep namespace on subcommands only when each command needs its own namespace scope.
