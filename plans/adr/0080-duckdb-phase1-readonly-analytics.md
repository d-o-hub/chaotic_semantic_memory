# ADR-0080: DuckDB Companion — Phase 1: Read-Only Analytics

## Status

Proposed (2026-05-15)

Tracks: GitHub Issue [#210](https://github.com/d-o-hub/chaotic_semantic_memory/issues/210), Phase 1.

Parent: ADR-0079 (Workspace Restructure for `csm-duckdb`).
Successor: ADR-0081 (Phase 2 — Parquet Export).

## Context and Problem Statement

After ADR-0079 establishes the companion crate skeleton, Phase 1 must deliver the smallest useful slice: **read-only ingestion of exported or persisted memory data into DuckDB**, with SQL-based inspection, summary statistics, and benchmark aggregation.

Phase 1 is read-only. It must never mutate the libSQL persistence file, the export payload, or any benchmark artifact.

## Decision Drivers

- Deliver immediate value: let operators run ad-hoc SQL against memory snapshots.
- Avoid coupling to live writes; the analytics path is offline OLAP, not OLTP.
- Keep the API surface tiny; one type per data source.
- Match repo conventions (LOC ≤ 500/file, ≥ 90% test:source ratio, clippy clean).

## Considered Options

1. **Direct libSQL → DuckDB attach** — Use DuckDB's SQLite scanner to attach `csm_memory.db` directly.
2. **Export-first ingestion** — Re-use the existing `ExportPayload` JSON path; load it into DuckDB tables.
3. **Hybrid** — Both paths, selectable via `enum DataSource`.

## Decision Outcome

Chosen: **Option 3 — Hybrid, with the export path as the default and best-supported route.**

The JSON export path is forward-compatible (we already version it via ADR-0016, ADR-0058). The libSQL attach path is convenient for ops users with a live database file but is best-effort: schema drift is the core crate's domain, not the companion's.

## Implementation

### Module Layout

```
crates/csm-duckdb/
├── Cargo.toml
├── README.md
├── AGENTS.md
└── src/
    ├── lib.rs            # public re-exports + crate-level docs
    ├── connection.rs     # DuckDB connection wrapper, in-memory by default
    ├── ingest_export.rs  # load ExportPayload (JSON) into duckdb tables
    ├── ingest_libsql.rs  # attach an existing csm libsql/sqlite file (read-only)
    ├── ingest_bench.rs   # ingest benchmarks/*.jsonl + summary.json
    ├── schema.rs         # canonical DuckDB schema DDL
    ├── stats.rs          # summary statistics (counts, percentiles, top-N)
    └── error.rs          # crate Error type, From<duckdb::Error>, etc.
```

Each file ≤ 300 LOC; total Phase 1 budget ≤ 1500 LOC of source + ≥ 1350 LOC of tests.

### Public API (Phase 1)

```rust
pub struct Analytics {
    conn: duckdb::Connection,
}

impl Analytics {
    /// In-memory analytics database.
    pub fn open_in_memory() -> Result<Self>;

    /// File-backed analytics database (DuckDB native format).
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self>;

    /// Load a JSON export payload (produced by `csm export`).
    pub fn load_export_json<P: AsRef<Path>>(&mut self, path: P) -> Result<IngestReport>;

    /// Attach a libSQL/SQLite database file in read-only mode and copy concepts.
    pub fn attach_libsql<P: AsRef<Path>>(&mut self, path: P) -> Result<IngestReport>;

    /// Load benchmark JSONL files (one record per line) into a table.
    pub fn load_benchmarks_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<IngestReport>;

    /// Run an arbitrary read-only SELECT query.
    pub fn query(&self, sql: &str) -> Result<duckdb::arrow::array::RecordBatch>;

    /// Convenience helpers for summary stats.
    pub fn concept_summary(&self) -> Result<ConceptSummary>;
    pub fn benchmark_summary(&self) -> Result<BenchmarkSummary>;
}
```

### Canonical DuckDB Schema (Phase 1)

```sql
CREATE TABLE concepts (
    id            VARCHAR PRIMARY KEY,
    text          VARCHAR,
    namespace     VARCHAR,
    created_at_us BIGINT,
    updated_at_us BIGINT,
    expires_at_us BIGINT,
    metadata_json JSON
);

CREATE TABLE associations (
    src_id   VARCHAR,
    dst_id   VARCHAR,
    strength DOUBLE,
    PRIMARY KEY (src_id, dst_id)
);

CREATE TABLE concept_versions (
    id          VARCHAR,
    version     INTEGER,
    text        VARCHAR,
    created_us  BIGINT,
    PRIMARY KEY (id, version)
);

CREATE TABLE benchmarks (
    suite     VARCHAR,
    name      VARCHAR,
    commit    VARCHAR,
    run_at_us BIGINT,
    p50_us    DOUBLE,
    p95_us    DOUBLE,
    p99_us    DOUBLE,
    extras    JSON
);
```

### Read-Only Guarantees

- `attach_libsql` uses DuckDB `ATTACH '...' AS csm (TYPE SQLITE, READ_ONLY)`.
- `load_export_json` reads the JSON file with `std::fs::File::open` (no write handle).
- The companion crate never imports or constructs a `libsql::Connection` itself; it only reads the file via DuckDB's SQLite scanner extension.
- A `forbid(unsafe_code)` lint is set at the crate root.

## Acceptance Criteria

- [ ] `cargo build -p csm-duckdb` succeeds on Linux/macOS.
- [ ] `cargo test -p csm-duckdb` covers:
  - JSON export ingest roundtrip (golden fixture under `tests/fixtures/`).
  - libSQL attach against a fixture file.
  - Benchmarks JSONL ingest from `benchmarks/` sample data.
  - `concept_summary()` and `benchmark_summary()` happy paths.
- [ ] Test:source ratio ≥ 90% for the new crate.
- [ ] All clippy lints pass at `-D warnings` with the same `clippy::map_unwrap_or` pedantic promotion as the core crate.
- [ ] No DuckDB type appears in the public API of `chaotic_semantic_memory`.
- [ ] README example shows: `Analytics::open_in_memory().load_export_json(...).query("SELECT count(*) FROM concepts")`.

## Out of Scope (Deferred to Later Phases)

- Parquet writers (ADR-0081).
- CLI integration (ADR-0082).
- Live streaming ingestion from a running framework.
- Write-back of analytics results into the core memory store.

## Risks

| Risk | Mitigation |
| --- | --- |
| DuckDB native build fails on a CI runner | Pin `duckdb = { features = ["bundled"] }`; restrict matrix to Linux + macOS for now. |
| libSQL schema drift breaks `attach_libsql` | Treat `attach_libsql` as best-effort; expose a `--schema-version` check on the source DB. |
| Export payload format changes silently | Version-check against `ExportPayload::SCHEMA_VERSION` and fail fast with a clear error. |

## Estimated Effort

- ~3-5 days of focused engineering for one author.
- Single PR, atomic, gated on ADR-0079 landing first.
