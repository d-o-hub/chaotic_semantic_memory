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
5. Verify LOC: every `src/*.rs` file must be ≤ 500 lines.

## Module Map

| File | Purpose | LOC |
|---|---|---|
| `src/lib.rs` | Crate root + prelude | ~63 |
| `src/error.rs` | `MemoryError` enum, `Result` alias | ~32 |
| `src/hyperdim.rs` | `HVec10240` (10240-bit vectors), bundle, bind, similarity | ~404 |
| `src/reservoir.rs` | Sparse ESN, `ChaoticReservoir`, spectral radius | ~469 |
| `src/singularity.rs` | `Concept`, `Singularity` store, similarity search | ~484 |
| `src/persistence.rs` | libSQL persistence, batch ops, per-op connections | ~500 |
| `src/framework.rs` | `ChaoticSemanticFramework`, builder, lifecycle | ~496 |
| `src/encoder.rs` | `TextEncoder`, deterministic text-to-hypervector | ~500 |
| `src/semantic_bridge.rs` | `CanonicalConcept`, `ConceptGraph`, `ScoreBreakdown` | ~400 |
| `src/bridge_retrieval.rs` | Bridge retrieval pipeline with expansion | ~387 |
| `src/bridge_persistence.rs` | Bridge persistence layer | ~300 |
| `src/retrieval/bm25.rs` | BM25 keyword search index | ~300 |
| `src/retrieval/hybrid.rs` | Hybrid BM25/HDC score fusion | ~200 |
| `src/wasm.rs` | WASM bindings (cfg-gated) | ~435 |
| `src/cli/` | CLI commands (inject, probe, query, index-*) | various |

## Key Conventions
- All public APIs return `Result<T, MemoryError>`.
- Tokio async for I/O (`persistence.rs`, `framework.rs`).
- Rayon for CPU parallelism, always behind `#[cfg(not(target_arch = "wasm32"))]`.
- libsql only (never `turso-client`).
- Reservoir spectral radius must stay in `[0.9, 1.1]`.
- `rand::rngs::StdRng` with `SeedableRng` for reproducibility in reservoir/tests.
- No magic numbers in production logic: use named constants and configurable env/config parameters for tunables.

## When Adding a New Module
- Add `pub mod name;` to `lib.rs`.
- Re-export key types in the `prelude` module if they're part of the public API.
- Add cfg gate if it uses Rayon or threading.
