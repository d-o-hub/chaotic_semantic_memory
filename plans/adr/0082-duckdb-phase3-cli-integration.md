# ADR-0082: DuckDB Companion — Phase 3: Optional CLI Integration

## Status

Proposed (2026-05-15)

Tracks: GitHub Issue [#210](https://github.com/d-o-hub/chaotic_semantic_memory/issues/210), Phase 3.

Parent: ADR-0079 (Workspace Restructure for `chaotic_semantic_memory_duckdb`).
Predecessors: ADR-0080 (Phase 1 — Read-Only Analytics), ADR-0081 (Phase 2 — Parquet Export).

## Context and Problem Statement

Phases 1 and 2 deliver a usable Rust API. Operators living in the terminal want one-liners. Phase 3 wires the companion crate into the `csm` binary as a non-default subcommand that is **only present when the DuckDB companion is enabled**.

The wiring must not pull DuckDB into the default `csm` build, must not change any existing subcommand's behavior, and must not surface DuckDB types in the core CLI's argument parser.

## Decision Drivers

- Default `csm` build stays slim (no `duckdb` C++ pulled in).
- Surface area lives entirely under one parent subcommand: `csm analytics …`.
- Keep argument parsing in the companion crate so the core CLI does not need DuckDB knowledge.
- Reuse `clap` derive in both crates without duplication.

## Considered Options

1. **Compile-time feature on the core crate** — `csm` gains `--features analytics` that pulls in the companion crate and exposes a `clap` subcommand built in the companion.
2. **Separate binary** — Ship `csm-analytics` as its own binary in the companion crate.
3. **Both** — Provide the separate binary by default and the integrated subcommand only when the feature is enabled.

## Decision Outcome

Chosen: **Option 3 — both, with the standalone binary as the supported default and the integrated subcommand as a convenience.**

Rationale:

- The standalone binary keeps the default `csm` build untouched and gives users a clean install path: `cargo install chaotic_semantic_memory_duckdb` ships `csm-analytics`.
- The integrated subcommand exists for power users who want `csm analytics …`; they opt in via `cargo install chaotic_semantic_memory --features analytics`.

This avoids forcing the choice on users while keeping defaults clean.

## Implementation

### Companion Crate Additions

```
crates/chaotic_semantic_memory_duckdb/
├── Cargo.toml             # adds [[bin]] csm-analytics, [features] cli
└── src/
    ├── cli/
    │   ├── mod.rs         # public `pub fn build_subcommand() -> clap::Command`
    │   ├── inspect.rs     # `csm-analytics inspect` and `csm analytics inspect`
    │   ├── export.rs      # `csm-analytics export` (Parquet, JSON)
    │   └── stats.rs       # `csm-analytics stats`
    └── bin/
        └── csm-analytics.rs  # standalone binary (≤ 50 LOC, calls into cli::)
```

`Cargo.toml`:

```toml
[features]
default = []
parquet = []
cli = ["dep:clap", "dep:anyhow"]

[dependencies]
clap = { version = "4", features = ["derive"], optional = true }
anyhow = { version = "1", optional = true }

[[bin]]
name = "csm-analytics"
path = "src/bin/csm-analytics.rs"
required-features = ["cli"]
```

### Core Crate Hook (Optional)

Add a tiny optional feature on the core crate:

```toml
# Cargo.toml (core)
[features]
analytics = ["dep:chaotic_semantic_memory_duckdb"]

[dependencies]
chaotic_semantic_memory_duckdb = { path = "crates/chaotic_semantic_memory_duckdb",
                                   features = ["cli", "parquet"], optional = true }
```

In `src/cli/mod.rs` (core CLI), add a `cfg`-gated subcommand:

```rust
#[cfg(feature = "analytics")]
fn analytics_subcommand() -> clap::Command {
    chaotic_semantic_memory_duckdb::cli::build_subcommand().name("analytics")
}
```

### Subcommand Surface

```
csm-analytics
├── inspect <db|export.json>          # opens REPL-like SQL prompt
├── query   <db|export.json> "<SQL>"  # one-shot SQL, prints table
├── stats   <db|export.json>          # concept_summary + benchmark_summary
└── export  <db|export.json>          # Parquet bundle (Phase 2)
    ├── --out <dir>
    ├── --compression zstd|snappy|none
    ├── --row-group-size <N>
    └── --partition-by <col>
```

Mirror under `csm analytics …` when the `analytics` feature is enabled.

### Output Conventions

- All commands accept `--format json|table|csv` (default: `table` to stdout).
- Exit codes: `0` success, `2` user input error, `3` ingest error, `4` export error.
- All commands emit a single-line `event=` log to stderr at INFO level for observability (matches existing `csm` conventions).

## Acceptance Criteria

- [ ] Default `cargo install chaotic_semantic_memory` does **not** install `csm-analytics`.
- [ ] `cargo install --path crates/chaotic_semantic_memory_duckdb --features cli,parquet` produces a working `csm-analytics` binary.
- [ ] `cargo install --path . --features analytics,cli` produces a `csm` binary that exposes `csm analytics --help`.
- [ ] Integration tests under `crates/chaotic_semantic_memory_duckdb/tests/cli_*.rs` cover `inspect`, `query`, `stats`, `export` against a fixture.
- [ ] `--help` output is snapshot-tested (e.g., via `insta`) so accidental drift is caught.
- [ ] No DuckDB types appear in `csm --help` when built without `analytics`.

## Out of Scope (Deferred)

- Interactive SQL REPL beyond a single prompt loop (defer to a later ADR if requested).
- Remote DuckDB endpoints (HTTP, MotherDuck).
- Authentication / RBAC for analytics commands.

## Risks

| Risk | Mitigation |
| --- | --- |
| `--help` output drift between `csm-analytics` and `csm analytics` | Single source: `cli::build_subcommand()`; integrated path renames it. Snapshot tests catch divergence. |
| Users confused by two installation paths | Document a decision tree in the companion `README.md`: "Want analytics only? Install the companion binary. Want one CLI? Use the feature." |
| `cargo install` with the `analytics` feature pulls native DuckDB unexpectedly | Explicit warning in core README + `cargo install` docs that `analytics` requires a C++ toolchain on some platforms. |

## Estimated Effort

- ~2 days after Phases 1 + 2 land.
- Single PR, atomic, gated on ADR-0081 merge.
