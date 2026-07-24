---
name: rust-development
description: "Implement or refactor Rust in this repository. Use when writing new modules, modifying existing source files, or adding features to the chaotic_semantic_memory crate."
---

# Rust Development

## Workflow
1. Read @AGENTS.md constraints.
2. Read the target file and its neighbors before editing.
3. Follow the patterns in `reference/codebase-patterns.md`.
4. Run `scripts/validate.sh` after changes.
5. Verify LOC: every `.rs` file in `src/` AND `crates/` must be ≤ 500 lines.
6. Run `cargo deny check` if Cargo.lock was modified (supply chain audit).

## Workspace Crates

| Crate | Purpose |
|---|---|
| `csm-core-lib` | HVec10240, encoder, reservoir, error types |
| `csm-memory` | Singularity store, concept builder, graph traversal, index (HNSW/LSH) |
| `csm-persistence` | libSQL persistence, migrations, concepts, versions |
| `csm-retrieval` | BM25, hybrid search, rerank, graph RAG |
| `csm-cli` | CLI binary, commands, git-local index |
| `csm-wasm` | WASM bindings, graph RAG wasm |
| `csm-embedding` | FastEmbed, HDC text, remote providers |
| `csm-duckdb` | DuckDB analytics, parquet export |
| `csm-chaos` | Chaotic hashing, hyperchaotic maps |
| `csm-traits` | Shared trait definitions |

## Root `src/` (framework integration layer)

| File | Purpose |
|---|---|
| `src/lib.rs` | Crate root + re-exports (~294 LOC) |
| `src/framework.rs` | `ChaoticSemanticFramework`, lifecycle |
| `src/framework_builder.rs` | Builder pattern for framework |
| `src/framework_persistence.rs` | Framework ↔ persistence glue |
| `src/framework_ops.rs` | Export operations |
| `src/framework_ops_import.rs` | Import operations |
| `src/semantic_bridge.rs` | `CanonicalConcept`, `ConceptGraph` |
| `src/bridge_retrieval.rs` | Bridge retrieval pipeline |
| `src/bridge_persistence.rs` | Bridge persistence layer |
| `src/wasm.rs` | WASM bindings (cfg-gated) |
| `src/persistence.rs` | libSQL persistence dispatch |

## Key Conventions
- All public APIs return `Result<T, MemoryError>`.
- Tokio async for I/O (`persistence.rs`, `framework.rs`).
- Rayon for CPU parallelism, always behind `#[cfg(not(target_arch = "wasm32"))]`.
- libsql only (never `turso-client`).
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- `rand::rngs::StdRng` with `SeedableRng` for reproducibility in reservoir/tests.
- No magic numbers in production logic: use named constants and configurable env/config parameters for tunables.
- **LOC gate applies to `crates/` too** — workspace crate source files at `crates/*/src/*.rs` have the same ≤ 500 LOC limit.
- **When adding a new workspace crate scope**, also add it to `commitlint.config.cjs` scope-enum.

## When Adding a New Module
- Add `pub mod name;` to `lib.rs`.
- Re-export key types in the `prelude` module if they're part of the public API.
- Add cfg gate if it uses Rayon or threading.
- If the file grows past ~400 LOC, proactively split (extract types, tests, or trait impls).
