#[cfg(feature = "parquet")]
mod tests {
    use chaotic_semantic_memory_duckdb::{Analytics, ParquetExportOptions};
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_parquet_export_roundtrip() {
        let analytics = Analytics::open_in_memory().unwrap();

        // Ingest some data
        analytics.conn.execute(
            "INSERT INTO concepts (id, text, namespace) VALUES ('c1', 'hello', 'ns1')",
            [],
        ).unwrap();
        analytics.conn.execute(
            "INSERT INTO concepts (id, text, namespace) VALUES ('c2', 'world', 'ns1')",
            [],
        ).unwrap();

        let dir = tempdir().unwrap();
        let opts = ParquetExportOptions::default();
        let report = analytics.export_concepts_parquet(dir.path().join("concepts.parquet"), &opts).unwrap();

        assert_eq!(report.rows_written, 2);
        assert!(report.bytes_written > 0);
        assert!(!report.sha256.is_empty());

        // Re-ingest into a fresh DuckDB
        let analytics2 = Analytics::open_in_memory().unwrap();
        analytics2.conn.execute(
            &format!("COPY concepts FROM '{}' (FORMAT PARQUET)", report.path.to_string_lossy()),
            [],
        ).unwrap();

        let count: i64 = analytics2.conn.query_row("SELECT count(*) FROM concepts", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_parquet_export_determinism() {
        let analytics = Analytics::open_in_memory().unwrap();
        analytics.conn.execute(
            "INSERT INTO concepts (id, text, namespace) VALUES ('c1', 'hello', 'ns1')",
            [],
        ).unwrap();

        let dir = tempdir().unwrap();
        let opts = ParquetExportOptions::default();

        let report1 = analytics.export_concepts_parquet(dir.path().join("concepts1.parquet"), &opts).unwrap();
        let report2 = analytics.export_concepts_parquet(dir.path().join("concepts2.parquet"), &opts).unwrap();

        assert_eq!(report1.sha256, report2.sha256);
    }

    #[test]
    fn test_export_all_bundle() {
        let analytics = Analytics::open_in_memory().unwrap();
        analytics.conn.execute("INSERT INTO concepts (id) VALUES ('c1')", []).unwrap();
        analytics.conn.execute("INSERT INTO associations (src_id, dst_id) VALUES ('c1', 'c2')", []).unwrap();
        analytics.conn.execute("INSERT INTO concept_versions (id, version) VALUES ('c1', 1)", []).unwrap();
        analytics.conn.execute("INSERT INTO benchmarks (suite, name) VALUES ('s1', 'n1')", []).unwrap();

        let dir = tempdir().unwrap();
        let opts = ParquetExportOptions::default();
        let bundle = analytics.export_all_parquet(dir.path(), &opts).unwrap();

        assert_eq!(bundle.concepts.rows_written, 1);
        assert_eq!(bundle.associations.rows_written, 1);
        assert_eq!(bundle.versions.rows_written, 1);
        assert_eq!(bundle.benchmarks.rows_written, 1);
        assert!(bundle.manifest_path.is_some());

        let manifest_content = fs::read_to_string(bundle.manifest_path.unwrap()).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content).unwrap();

        assert_eq!(manifest["schema_version"], 1);
        assert!(manifest["files"]["concepts.parquet"].is_object());
    }

    #[test]
    #[ignore]
    fn test_perf_1m_concepts_memory() {
        // This test is ignored by default as per ADR-0081
        let analytics = Analytics::open_in_memory().unwrap();

        // Use DuckDB to generate 1M concepts efficiently
        analytics.conn.execute_batch("
            INSERT INTO concepts (id, text, namespace)
            SELECT 'c' || i, 'text' || i, 'ns'
            FROM range(1, 1000001) t(i)
        ").unwrap();

        let dir = tempdir().unwrap();
        let opts = ParquetExportOptions::default();
        let _report = analytics.export_concepts_parquet(dir.path().join("heavy.parquet"), &opts).unwrap();

        // Verification of RSS would be platform dependent,
        // but we can at least ensure it completes.
    }
}
