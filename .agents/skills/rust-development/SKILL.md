---
name: rust-development
description: "Implement or refactor Rust in this repository. Use when writing new modules, modifying existing source files, or adding features to the chaotic_semantic_memory crate."
---

# Rust Development

## Workflow
1. Read `AGENTS.md` and the target file and neighbors before editing.
2. Follow the patterns in `reference/codebase-patterns.md`.
3. On a fresh checkout, run `scripts/install-hooks.sh` before validation.
4. Run both `scripts/validate.sh` and `scripts/harness-check.sh all` after changes; do not treat one as a substitute for the other.
5. Verify the changed behavior with a focused smoke scenario in addition to tests.
6. Keep every Rust file under `src/` and `crates/` at or below 500 LOC.
7. If production logic in `src/` or `crates/` changes, add mutation-visible unit assertions in the library target; integration-only tests do not count for the mutation gate.
8. Run `cargo deny --locked check` when Cargo manifests or `Cargo.lock` change.
9. For exported symbol changes, query LSP references before editing and update every caller.

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
- **LOC gate applies to `crates/` too** — workspace crate source files at `crates/*/src/*.rs` have the same ≤ 500 LOC limit.
- **When adding a new workspace crate scope**, also add it to `commitlint.config.cjs` scope-enum.
- Preserve rationale comments; extract a child module rather than stripping comments to satisfy the LOC limit.

## When Adding a New Module
- Add `pub mod name;` to `lib.rs`.
- Re-export key types in the `prelude` module if they're part of the public API.
- Add cfg gate if it uses Rayon or threading.
- If the file grows past ~400 LOC, proactively split (extract types, tests, or trait impls).
