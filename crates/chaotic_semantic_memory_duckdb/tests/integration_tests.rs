use chaotic_semantic_memory_duckdb::{Analytics, AnalyticsError};
use std::path::PathBuf;

fn get_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_load_export_json() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let report = analytics
        .load_export_json(get_fixture("export.json"))
        .unwrap();

    assert_eq!(report.concepts_loaded, 2);
    assert_eq!(report.associations_loaded, 1);

    let summary = analytics.concept_summary().unwrap();
    assert_eq!(summary.total_concepts, 2);
    assert_eq!(summary.total_associations, 1);
    assert!(summary.namespaces.contains(&"default".to_string()));
    assert!(summary.namespaces.contains(&"science".to_string()));
}

#[test]
fn test_load_benchmarks() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let report = analytics.load_benchmarks_dir(dir).unwrap();

    assert_eq!(report.benchmarks_loaded, 2); // 2 lines in results.jsonl

    let summary = analytics.benchmark_summary().unwrap();
    assert_eq!(summary.total_runs, 2);
    assert_eq!(summary.avg_p50_us, 15250.0); // (10.5 + 20.0) / 2 * 1000
}

#[test]
fn test_attach_libsql() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let report = analytics.attach_libsql(get_fixture("csm.db")).unwrap();

    assert_eq!(report.concepts_loaded, 1);
    assert_eq!(report.associations_loaded, 1);

    let summary = analytics.concept_summary().unwrap();
    assert_eq!(summary.total_concepts, 1);
}

#[test]
fn test_query() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    analytics
        .load_export_json(get_fixture("export.json"))
        .unwrap();

    let batches = analytics.query("SELECT count(*) FROM concepts").unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), 1);
}

#[test]
fn test_open_file_backed() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("test.duckdb");
    {
        let _analytics = Analytics::open(&db_path).unwrap();
    }
    assert!(db_path.exists());
}

#[test]
fn test_error_missing_file() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let res = analytics.load_export_json("nonexistent.json");
    assert!(matches!(res, Err(AnalyticsError::Io(_))));
}

#[test]
fn test_error_invalid_json() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), "{ invalid json").unwrap();
    let res = analytics.load_export_json(temp.path());
    assert!(matches!(res, Err(AnalyticsError::Json(_))));
}

#[test]
fn test_empty_benchmarks_dir() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let report = analytics.load_benchmarks_dir(temp.path()).unwrap();
    assert_eq!(report.benchmarks_loaded, 0);
}

#[test]
fn test_query_no_results() {
    let analytics = Analytics::open_in_memory().unwrap();
    let batches = analytics
        .query("SELECT * FROM concepts WHERE id = 'none'")
        .unwrap();
    assert!(batches.is_empty());
}

#[test]
fn test_query_invalid_sql() {
    let analytics = Analytics::open_in_memory().unwrap();
    let res = analytics.query("SELECT INVALID FROM table");
    assert!(matches!(res, Err(AnalyticsError::DuckDb(_))));
}

#[test]
fn test_stats_empty() {
    let analytics = Analytics::open_in_memory().unwrap();
    let c_summary = analytics.concept_summary().unwrap();
    assert_eq!(c_summary.total_concepts, 0);
    assert_eq!(c_summary.total_associations, 0);
    assert!(c_summary.namespaces.is_empty());

    let b_summary = analytics.benchmark_summary().unwrap();
    assert_eq!(b_summary.total_runs, 0);
    assert_eq!(b_summary.avg_p50_us, 0.0);
    assert!(b_summary.suites.is_empty());
}

#[test]
fn test_version() {
    assert_eq!(chaotic_semantic_memory_duckdb::version(), "0.1.0");
}
