# ADR-0081: DuckDB Companion — Phase 2: Parquet Export

## Status

Proposed (2026-05-15)

Tracks: GitHub Issue [#210](https://github.com/d-o-hub/chaotic_semantic_memory/issues/210), Phase 2.

Parent: ADR-0079 (Workspace Restructure for `csm-duckdb`).
Predecessor: ADR-0080 (Phase 1 — Read-Only Analytics).
Successor: ADR-0082 (Phase 3 — Optional CLI Integration).

## Context and Problem Statement

Once Phase 1 lands, the companion crate has the data in DuckDB tables. Phase 2 must let users **export memory snapshots, benchmark results, and diagnostic data to Parquet** so the data can be consumed by downstream tools (Polars, Spark, DuckDB CLI, BI dashboards, ML training pipelines).

Parquet is the right pivot because:
- It is the de-facto OLAP interchange format.
- DuckDB writes it natively (`COPY ... TO ... (FORMAT PARQUET)`); no extra dependency required.
- It preserves types (timestamps, JSON, nested) better than CSV.

## Decision Drivers

- Use DuckDB's built-in writer; do not pull `arrow2`, `parquet2`, or another writer crate.
- Keep partitioning sensible for typical sizes (≤ 10 M concepts) without forcing it.
- Make exports reproducible (deterministic ordering, stable column names).
- Ship hashed metadata for verifiability (run-id, source path, schema version).

## Considered Options

1. **DuckDB native `COPY ... TO PARQUET`** — Single SQL statement per dataset.
2. **Pull `arrow` + `parquet` crates** — Build batches manually.
3. **Two-step: dump to CSV then convert externally** — Lossy, rejected immediately.

## Decision Outcome

Chosen: **Option 1 — DuckDB native Parquet writer.**

Zero new dependencies, supported on every DuckDB platform, handles ZSTD/SNAPPY compression natively, and produces files readable by every Parquet consumer.

Option 2 is rejected because it duplicates code DuckDB already maintains and would force the companion crate to track two ecosystems (DuckDB + Arrow).

## Implementation

### Module Additions

```
crates/csm-duckdb/src/
├── export_parquet.rs   # NEW: write helpers
└── manifest.rs         # NEW: run manifest + checksum
```

Each file ≤ 300 LOC.

### Public API (Phase 2)

```rust
#[derive(Clone, Debug)]
pub struct ParquetExportOptions {
    pub compression: ParquetCompression, // Zstd (default), Snappy, None
    pub row_group_size: usize,           // default 122_880 (DuckDB default)
    pub partition_by: Option<String>,    // e.g. Some("namespace")
    pub include_manifest: bool,          // default true
}

pub enum ParquetCompression { Zstd, Snappy, None }

impl Analytics {
    pub fn export_concepts_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport>;

    pub fn export_associations_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport>;

    pub fn export_versions_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport>;

    pub fn export_benchmarks_parquet<P: AsRef<Path>>(
        &self,
        out_path: P,
        opts: &ParquetExportOptions,
    ) -> Result<ExportReport>;

    /// Convenience: writes all four datasets into a directory and emits a manifest.json.
    pub fn export_all_parquet<P: AsRef<Path>>(
        &self,
        out_dir: P,
        opts: &ParquetExportOptions,
    ) -> Result<BundleReport>;
}

pub struct ExportReport {
    pub rows_written: u64,
    pub bytes_written: u64,
    pub path: PathBuf,
    pub sha256: String,
}

pub struct BundleReport {
    pub concepts: ExportReport,
    pub associations: ExportReport,
    pub versions: ExportReport,
    pub benchmarks: ExportReport,
    pub manifest_path: PathBuf,
}
```

### SQL Generated (Conceptual)

```sql
COPY (
    SELECT id, text, namespace, created_at_us, updated_at_us, expires_at_us, metadata_json
    FROM concepts
    ORDER BY id
) TO 'concepts.parquet' (
    FORMAT PARQUET,
    COMPRESSION ZSTD,
    ROW_GROUP_SIZE 122880
);
```

For partitioned exports:

```sql
COPY (SELECT * FROM concepts ORDER BY namespace, id)
TO 'concepts/'
(FORMAT PARQUET, PARTITION_BY (namespace), COMPRESSION ZSTD);
```

### Manifest Format (`manifest.json`)

```json
{
  "schema_version": 1,
  "generator": "csm-duckdb 0.1.0",
  "core_crate_version": "0.3.5",
  "run_id": "ulid-...",
  "exported_at": "2026-05-15T12:00:00Z",
  "files": {
    "concepts.parquet":     {"rows": 12345, "bytes": 678901, "sha256": "..."},
    "associations.parquet": {"rows": 6789,  "bytes": 23456,  "sha256": "..."},
    "versions.parquet":     {"rows": 432,   "bytes": 9876,   "sha256": "..."},
    "benchmarks.parquet":   {"rows": 87,    "bytes": 5432,   "sha256": "..."}
  },
  "options": {"compression": "Zstd", "row_group_size": 122880, "partition_by": null}
}
```

### Determinism

- All exports use explicit `ORDER BY` on the primary key.
- Compression defaults to ZSTD level 3 (DuckDB default) for reproducibility.
- Manifest SHA-256 is computed over the file bytes after the writer flushes.

## Acceptance Criteria

- [ ] `cargo test -p csm-duckdb --features parquet` covers:
  - Roundtrip: ingest fixture → export Parquet → re-ingest into a fresh DuckDB → row counts match.
  - Determinism: two consecutive exports of the same data produce byte-identical Parquet (or at minimum identical SHA-256 of sorted row groups; document if DuckDB writes non-deterministic file metadata).
  - Manifest schema validation against a JSON-schema fixture.
- [ ] No new top-level dependency (no `arrow`, no `parquet`).
- [ ] Test:source ratio for new files ≥ 90%.
- [ ] Documentation: `crates/csm-duckdb/README.md` shows a Polars + Python read example.
- [ ] Memory ceiling: exporting 1 M concepts stays under 1 GB RSS on a CI runner (verified by an `--ignored` test marked `#[ignore]` that runs locally).

## Out of Scope (Deferred to Phase 3 or later)

- CLI integration (`csm analytics export-parquet …`) — see ADR-0082.
- Streaming export to S3/GCS (would require object-store deps).
- Live exporters that tail a running framework.

## Risks

| Risk | Mitigation |
| --- | --- |
| Non-deterministic Parquet file metadata (compression dictionaries, timestamps) | Document expected determinism level; manifest hashes individual file bytes, accept that file SHA may differ across DuckDB versions. |
| Large row groups blow memory | Default `ROW_GROUP_SIZE` matches DuckDB's; expose option for tuning. |
| Partitioned exports create many small files | Document the trade-off; recommend `partition_by` only when downstream tooling expects Hive layout. |

## Estimated Effort

- ~2-3 days after Phase 1 lands.
- Single PR, atomic, gated on ADR-0080 merge.
