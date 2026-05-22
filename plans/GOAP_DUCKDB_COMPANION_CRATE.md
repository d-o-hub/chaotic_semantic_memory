# GOAP Action Plan: DuckDB Companion Crate (Issue #210)

> **Tracks**: GitHub Issue [#210](https://github.com/d-o-hub/chaotic_semantic_memory/issues/210) — *feat: add optional `chaotic_semantic_memory_duckdb` companion crate for analytics and export*
>
> **ADRs**:
> - ADR-0079 — Workspace Restructure for Companion Crate (parent)
> - ADR-0080 — Phase 1: Read-Only Analytics
> - ADR-0081 — Phase 2: Parquet Export
> - ADR-0082 — Phase 3: Optional CLI Integration

---

## Current State (from `plans/GOAP_STATE.md`)

```yaml
project_initialized: true
core_modules_created: true
tests_passing: true
benchmarks_exist: true
wasm_compiles: true
binary_built: true
ci_all_checks_passed: true
loc_gate_verified: true
coverage_ratio_current: 93   # ≥ 90% target
duckdb_companion_crate_present: false
duckdb_workspace_layout: false
duckdb_phase1_readonly_analytics: false
duckdb_phase2_parquet_export: false
duckdb_phase3_cli_integration: false
```

## Target State

```yaml
duckdb_companion_crate_present: true
duckdb_workspace_layout: true
duckdb_phase1_readonly_analytics: true
duckdb_phase2_parquet_export: true
duckdb_phase3_cli_integration: true
core_default_dependency_unchanged: true     # no duckdb in default cargo build
core_wasm32_unaffected: true                # wasm32 path still green
core_public_api_no_duckdb_types: true       # API boundary clean
docs_two_crate_model_documented: true       # README explains core vs analytics
```

---

## Sub-Issue Decomposition

Issue #210 is split into four atomic sub-issues. Each maps 1:1 to an ADR and to one PR.

| Sub-issue | ADR | Phase | Estimated PR Size |
| --- | --- | --- | --- |
| **#210-A** Workspace restructure + skeleton companion crate | ADR-0079 | 0 (foundation) | ~300 LOC + CI matrix update |
| **#210-B** Phase 1 — Read-only analytics ingest + SQL | ADR-0080 | 1 | ~1500 LOC + ~1400 LOC tests |
| **#210-C** Phase 2 — Parquet export + manifest | ADR-0081 | 2 | ~600 LOC + ~600 LOC tests |
| **#210-D** Phase 3 — `csm-analytics` binary + optional `csm analytics` subcommand | ADR-0082 | 3 | ~500 LOC + ~500 LOC tests |

Each sub-issue has acceptance criteria copied from its ADR and links back to #210 as parent.

---

## Actions (Ordered by Cost / Dependency)

### Action 1 — Open sub-issues #210-A..D on GitHub (cost: 1)

**Preconditions:** issue #210 exists; ADRs 0079-0082 written.
**Effects:** sub-issues opened; cross-linked to #210.

Use `gh issue create --repo d-o-hub/chaotic_semantic_memory` for each sub-issue with a `Parent: #210` line in the body.

---

### Action 2 — Land ADR-0079 workspace restructure (cost: 3)

**Preconditions:** sub-issue #210-A open.
**Effects:**
- `duckdb_companion_crate_present = true`
- `duckdb_workspace_layout = true`

Steps:

1. Create branch `feat/duckdb-companion-workspace`.
2. Add root `[workspace]` table to `Cargo.toml` with members `[".", "benchmarks", "crates/chaotic_semantic_memory_duckdb"]`.
3. Create `crates/chaotic_semantic_memory_duckdb/{Cargo.toml,README.md,AGENTS.md,src/lib.rs}` — empty stub that compiles.
4. Add CI job `cargo test -p chaotic_semantic_memory_duckdb` (Linux only) to `.github/workflows/ci.yml`.
5. Verify `cargo build` (default features) does not pull `duckdb` (run `cargo tree --no-default-features | grep -i duckdb || true`).
6. Verify `cargo build --target wasm32-unknown-unknown` for the core crate still succeeds.
7. Run `./scripts/validate.sh`.
8. Open PR, wait for CI, squash-merge.

Validation gates: `cargo check --quiet && cargo test --all-features --quiet && cargo fmt --check && cargo clippy --quiet -- -D warnings`.

---

### Action 3 — Implement ADR-0080 Phase 1 read-only analytics (cost: 5)

**Preconditions:** Action 2 merged; `duckdb_workspace_layout = true`.
**Effects:** `duckdb_phase1_readonly_analytics = true`.

Steps:

1. Branch `feat/duckdb-phase1-readonly-analytics`.
2. Add modules per ADR-0080: `connection.rs`, `ingest_export.rs`, `ingest_libsql.rs`, `ingest_bench.rs`, `schema.rs`, `stats.rs`, `error.rs`.
3. Create `tests/fixtures/` with a tiny golden export JSON and a tiny libSQL DB (≤ 50 rows total, committed via `git lfs` if > 100 KB).
4. Add tests:
   - `tests/ingest_export.rs`
   - `tests/ingest_libsql.rs`
   - `tests/ingest_benchmarks.rs`
   - `tests/stats_summary.rs`
5. Verify test:source ratio ≥ 90%.
6. Run validation gates + LOC gate (≤ 500 per file, ≤ 300 budgeted by ADR).
7. Open PR, link sub-issue #210-B, wait for CI, squash-merge.

---

### Action 4 — Implement ADR-0081 Phase 2 Parquet export (cost: 4)

**Preconditions:** Action 3 merged; `duckdb_phase1_readonly_analytics = true`.
**Effects:** `duckdb_phase2_parquet_export = true`.

Steps:

1. Branch `feat/duckdb-phase2-parquet-export`.
2. Add `export_parquet.rs`, `manifest.rs`, and the `parquet` feature flag.
3. Add roundtrip test: ingest fixture → export Parquet → re-ingest → row counts match.
4. Add manifest schema fixture under `tests/fixtures/manifest.schema.json` and validate generated manifest.
5. Add `#[ignore]` large-data perf test (1 M concepts, < 1 GB RSS).
6. Update companion `README.md` with a Polars + Python read example.
7. Run validation gates.
8. PR → CI → merge.

---

### Action 5 — Implement ADR-0082 Phase 3 CLI integration (cost: 3)

**Preconditions:** Action 4 merged; `duckdb_phase2_parquet_export = true`.
**Effects:**
- `duckdb_phase3_cli_integration = true`
- `docs_two_crate_model_documented = true`

Steps:

1. Branch `feat/duckdb-phase3-cli-integration`.
2. Add companion `cli/` module with `inspect`, `query`, `stats`, `export` subcommands.
3. Add `[[bin]] csm-analytics` gated behind `cli` feature.
4. Add optional `analytics` feature on the core crate that pulls the companion + exposes a `csm analytics` subcommand.
5. Add snapshot tests on `--help` output (use `insta`).
6. Update root `README.md` with the two-crate decision tree.
7. Verify default `cargo install chaotic_semantic_memory` does NOT install `csm-analytics` and does NOT pull DuckDB.
8. Run validation gates.
9. PR → CI → merge.

---

### Action 6 — Update GOAP_STATE and ADR_REGISTRY (cost: 1)

**Preconditions:** Action 5 merged.
**Effects:** state synced, registry shows ADR-0079..0082 as Implemented.

1. Append ADR-0079..0082 rows to `plans/ADR_REGISTRY.md`.
2. Update `plans/GOAP_STATE.md`:
   - `action_last_completed: duckdb_phase3_cli_integration`
   - All five `duckdb_*` flags = `true`.
   - Bump `tests_count` and `coverage_ratio_current`.
3. Add a learning to `progress/LEARNINGS.md` if the workspace migration produced any non-obvious patterns.

---

## Plan Diagram

```diagram
                                   ╭───────────────────────────╮
                                   │  Issue #210 (parent)      │
                                   ╰────────────┬──────────────╯
                                                │ split
              ╭─────────────────┬───────────────┼───────────────┬────────────────╮
              ▼                 ▼               ▼               ▼                ▼
        ╭───────────╮     ╭───────────╮   ╭───────────╮   ╭───────────╮   ╭───────────╮
        │ #210-A    │     │ #210-B    │   │ #210-C    │   │ #210-D    │   │ State sync│
        │ ADR-0079  │ ──▶ │ ADR-0080  │──▶│ ADR-0081  │──▶│ ADR-0082  │──▶│ ADR/GOAP  │
        │ workspace │     │ Phase 1   │   │ Phase 2   │   │ Phase 3   │   │ updates   │
        ╰───────────╯     ╰───────────╯   ╰───────────╯   ╰───────────╯   ╰───────────╯
         Action 2          Action 3        Action 4        Action 5        Action 6
```

---

## Hard Constraints (Must Not Violate)

- **Default-build cleanliness**: `cargo build` (no `--features`) MUST NOT pull `duckdb`.
- **WASM safety**: `cargo build --target wasm32-unknown-unknown` for the core crate MUST keep working at every step.
- **API boundary**: no `duckdb::*` type may appear in any `pub fn` of `chaotic_semantic_memory`.
- **One-way dependency**: companion → core only; never reversed.
- **LOC gate**: every new `.rs` file ≤ 500 LOC (ADRs 0080-0082 budget tighter).
- **Coverage**: test:source ratio for the companion crate ≥ 90%.
- **No direct push to `main`**; always branch → PR → CI green → squash-merge.

---

## Verification Checklist (End-to-End)

After Action 5 merges, run:

```bash
# Default build is unchanged
cargo build --quiet
cargo tree --no-default-features --prefix=none | grep -i duckdb && echo "FAIL: duckdb leaked into defaults" || echo "OK: no duckdb in defaults"

# WASM still green
cargo build --target wasm32-unknown-unknown --quiet

# Companion crate works end-to-end
cargo test -p chaotic_semantic_memory_duckdb --all-features --quiet

# CLI surface
cargo run -p chaotic_semantic_memory_duckdb --features cli,parquet --bin csm-analytics -- --help
cargo run --features analytics -- analytics --help    # only when analytics feature is on

# Real usage
csm export -o /tmp/export.json --database /tmp/csm.db
csm-analytics query /tmp/export.json "SELECT count(*) FROM concepts"
csm-analytics export /tmp/export.json --out /tmp/parquet --compression zstd
ls /tmp/parquet/*.parquet /tmp/parquet/manifest.json
```

All commands must succeed with exit code `0` and produce non-empty output.

---

## Risk Log

| Risk | Action | Mitigation |
| --- | --- | --- |
| Workspace conversion breaks `cargo install chaotic_semantic_memory` | 2 | Keep core crate at repo root for Stage A; only move under `crates/` in a future ADR. |
| DuckDB native build fails on Windows CI | 3 | Restrict companion CI matrix to Linux + macOS initially; document Windows status. |
| Parquet writer produces non-deterministic output | 4 | Hash file bytes in manifest; document that determinism is best-effort across DuckDB versions. |
| `--help` output drifts between standalone and integrated CLI | 5 | Single `build_subcommand()` source + `insta` snapshot tests. |

---

## Out of Scope (Future ADRs)

- Streaming exports to S3/GCS (would need `object_store`).
- Live tail of a running framework into DuckDB.
- MotherDuck / remote DuckDB endpoints.
- Additional companion crates (`_lance`, `_qdrant`) — would trigger Stage B of the workspace migration.
