# ADR-0079: Workspace Restructure for `chaotic_semantic_memory_duckdb` Companion Crate

## Status

Proposed (2026-05-15)

Tracks: GitHub Issue [#210](https://github.com/d-o-hub/chaotic_semantic_memory/issues/210)

Related ADRs:
- ADR-0001 (Use libSQL for Persistence) — confirms libSQL stays the operational store.
- ADR-0016 (Export/Import Migration) — shared export schema becomes the bridge to DuckDB.
- ADR-0038 (Cargo.toml Modernization) — workspace conventions.
- ADR-0067 (MCP Server) — precedent for optional, feature-gated subsystem.

Supersedes / Superseded by: none.

Successor ADRs:
- ADR-0080 (Phase 1 — Read-only Analytics)
- ADR-0081 (Phase 2 — Parquet Export)
- ADR-0082 (Phase 3 — Optional CLI Integration)

## Context and Problem Statement

Issue #210 requests adding DuckDB-powered analytics, Parquet export, and SQL inspection to the project. The core `chaotic_semantic_memory` crate already carries a wide load: core library, `cdylib` for FFI, the `csm` CLI binary, optional `libsql` persistence, parallel execution, embedding integrations (HTTP + ONNX), ANN backends (HNSW/LSH), MCP support, and `wasm32` builds.

Adding DuckDB directly would:

- Inflate the default dependency graph for users who only consume the core memory engine.
- Break or complicate the `wasm32` target (DuckDB is native-only, with C++ build requirements).
- Multiply CI feature combinations (`persistence × parallel × wasm × duckdb …`).
- Mix OLAP analytics types into the public API of an OLTP-style memory engine.

## Decision Drivers

- Keep the default `chaotic_semantic_memory` build lean, fast to compile, and `wasm32`-clean.
- Preserve a clean public API boundary: the analytics path must be opt-in and live in its own crate.
- Allow analytics to evolve iteratively (read → export → CLI integration) without churn in the core crate.
- Avoid duplicating shared types (export payload, benchmark records, snapshot schemas).
- Stay aligned with existing repo conventions (LOC ≤ 500/file, ADR-driven changes, atomic PRs).

## Considered Options

1. **Single-crate, feature-gated** — Add `duckdb` to the existing crate behind `--features duckdb`.
2. **Sibling crate at repo root** — Add a top-level `chaotic_semantic_memory_duckdb/` package.
3. **Cargo workspace with `crates/` directory** — Convert the repo into a workspace and place the companion crate at `crates/chaotic_semantic_memory_duckdb/`.

## Decision Outcome

Chosen: **Option 3 — Cargo workspace with `crates/` directory.**

Move toward a small workspace layout. The core crate stays in place (or moves to `crates/chaotic_semantic_memory/`) and the companion lives at `crates/chaotic_semantic_memory_duckdb/`. The companion depends on the core crate; **never the other way around**.

### Justification

- Option 1 violates the "core stays lean" requirement and breaks `wasm32` cleanly only via complex `cfg` gates.
- Option 2 works but does not scale if more companion crates are added (e.g., `_lance`, `_qdrant`).
- Option 3 is the standard Rust convention; gives a single `cargo build --workspace` and isolates failures.

### Migration Strategy

To minimize blast radius:

1. **Stage A (non-breaking):** Keep the core crate at the repo root for one release cycle. Add `crates/chaotic_semantic_memory_duckdb/` as a sibling and declare a virtual `[workspace]` in the root `Cargo.toml`.
2. **Stage B (optional, later release):** If a second companion crate is added, move the core crate into `crates/chaotic_semantic_memory/` in a dedicated migration ADR.

This staged approach avoids a disruptive path change for downstream consumers.

## Implementation Sketch

```diagram
╭───────────────────────────╮
│ chaotic_semantic_memory   │   (core, unchanged default features)
│  ├─ src/                  │
│  ├─ benches/              │
│  ├─ Cargo.toml            │
│  └─ wasm32 OK             │
╰────────────┬──────────────╯
             │ depends-on (one-way)
             ▼
╭────────────────────────────────────╮
│ crates/                            │
│  └─ chaotic_semantic_memory_duckdb │
│      ├─ src/                       │
│      ├─ tests/                     │
│      ├─ Cargo.toml                 │
│      └─ AGENTS.md                  │
╰────────────────────────────────────╯
```

Root `Cargo.toml` additions:

```toml
[workspace]
members = [
    ".",
    "benchmarks",
    "crates/chaotic_semantic_memory_duckdb",
]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/d-o-hub/chaotic_semantic_memory"
```

Companion crate `Cargo.toml`:

```toml
[package]
name = "chaotic_semantic_memory_duckdb"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Optional analytics, Parquet export, and SQL inspection for chaotic_semantic_memory"

[dependencies]
chaotic_semantic_memory = { path = "../..", default-features = false, features = ["persistence"] }
duckdb = { version = "1.x", features = ["bundled"] }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
anyhow = "1"

[features]
default = []
parquet = []   # toggles Phase 2 export utilities
```

### Shared Types

To avoid duplication, the core crate must expose (already exposes most via the `export` module):

- `ExportPayload` / `BinaryExportPayload` — concept snapshot.
- `Concept`, `ConceptMetadata`, `Association`.
- Benchmark result schema (currently in `benchmarks/`).

If any of these are not `pub`, a follow-up PR to the core crate will re-export them under `chaotic_semantic_memory::analytics_schema` (no new dependencies).

## Acceptance Criteria

- [ ] `cargo build` (default features) does **not** pull `duckdb`.
- [ ] `cargo build --workspace` builds both crates on Linux and macOS.
- [ ] `cargo build --target wasm32-unknown-unknown` for the core crate is unaffected.
- [ ] Companion crate has its own `AGENTS.md` and `README.md`.
- [ ] Root `README.md` documents the two-crate model under "Optional analytics".
- [ ] CI matrix adds a single job: `cargo test -p chaotic_semantic_memory_duckdb` (Linux only).
- [ ] No DuckDB types appear in the public API of `chaotic_semantic_memory`.

## Pros and Cons

### Pros
- Default users see no change; opt-in path is explicit.
- Clean dependency direction: companion → core.
- Failure isolation in CI; native-only DuckDB issues can never break `wasm32`.
- Sets the precedent for future companion crates.

### Cons
- Repo now has a workspace; minor mental overhead for new contributors.
- Shared schemas must be kept stable; breaking the export model affects both crates.

## Follow-ups

- ADR-0080 details Phase 1 (read-only analytics).
- ADR-0081 details Phase 2 (Parquet export).
- ADR-0082 details Phase 3 (optional CLI subcommand wiring).
