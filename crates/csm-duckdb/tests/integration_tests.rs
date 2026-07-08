use csm_duckdb::{Analytics, AnalyticsError};
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

    assert_eq!(report.benchmarks_loaded, 2);

    let summary = analytics.benchmark_summary().unwrap();
    assert_eq!(summary.total_runs, 2);
    assert!(summary.avg_p50_us > 0.0);
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

    let rows = analytics
        .query("SELECT count(*) as total FROM concepts")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["total"], 2);
}

#[test]
fn test_error_missing_file() {
    let mut analytics = Analytics::open_in_memory().unwrap();
    let res = analytics.load_export_json("nonexistent.json");
    assert!(matches!(res, Err(AnalyticsError::Io(_))));
}

#[test]
fn test_query_invalid_sql() {
    let analytics = Analytics::open_in_memory().unwrap();
    let res = analytics.query("SELECT INVALID FROM table");
    assert!(res.is_err());
}

#[test]
fn test_query_restricted() {
    let analytics = Analytics::open_in_memory().unwrap();
    let res = analytics.query("DROP TABLE concepts");
    assert!(res.is_err());
}
