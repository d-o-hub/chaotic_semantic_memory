#![cfg(feature = "cli")]

use csm_duckdb::cli::CliOutputFormat;
use csm_duckdb::schema::SCHEMA_DDL;
use duckdb::Connection;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_help_snapshots() {
    use clap::CommandFactory;

    #[derive(clap::Parser)]
    #[command(name = "csm-analytics")]
    struct Cli {
        #[command(subcommand)]
        command: csm_duckdb::cli::AnalyticsCommand,
    }

    let mut cmd = Cli::command();
    let help = cmd.render_help().to_string();
    insta::assert_snapshot!(help);
}

#[tokio::test]
async fn test_stats_command() {
    // A NamedTempFile is created empty on disk, and DuckDB refuses to open an
    // existing 0-byte file ("exists, but it is not a valid DuckDB database
    // file"). Hold a TempDir instead and hand DuckDB a path it can create.
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("analytics.duckdb");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    drop(conn);

    let analytics = csm_duckdb::Analytics::open(&db_path).unwrap();
    // Just verify it doesn't crash and returns OK
    csm_duckdb::cli::stats::run(&analytics, &CliOutputFormat::Table)
        .await
        .unwrap();
    csm_duckdb::cli::stats::run(&analytics, &CliOutputFormat::Json)
        .await
        .unwrap();
}

#[cfg(feature = "parquet")]
#[tokio::test]
async fn test_export_command() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("analytics.duckdb");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    drop(conn);

    let out_dir = tempfile::tempdir().unwrap();

    let cmd = csm_duckdb::cli::AnalyticsCommand::Export(csm_duckdb::cli::ExportArgs {
        input: db_path,
        out: out_dir.path().to_path_buf(),
        #[cfg(feature = "parquet")]
        compression: csm_duckdb::export_parquet::ParquetCompression::None,
        row_group_size: 1000,
        partition_by: None,
    });

    csm_duckdb::cli::run_analytics(cmd).await.unwrap();

    // Verify some files were created
    assert!(out_dir.path().join("concepts.parquet").exists());
}

#[tokio::test]
async fn test_query_command() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("analytics.duckdb");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    conn.execute(
        "INSERT INTO concepts (id, namespace) VALUES (?, ?)",
        duckdb::params!["c1", "ns1"],
    )
    .unwrap();
    drop(conn);

    let analytics = csm_duckdb::Analytics::open(&db_path).unwrap();
    csm_duckdb::cli::query::run(
        &analytics,
        "SELECT * FROM concepts",
        &CliOutputFormat::Table,
    )
    .await
    .unwrap();
    csm_duckdb::cli::query::run(&analytics, "SELECT * FROM concepts", &CliOutputFormat::Json)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_export_json_input() {
    // `open_analytics` routes by the `.json` extension; a NamedTempFile's random
    // name has none, so it fell through to opening the JSON as a database.
    let mut temp = tempfile::Builder::new().suffix(".json").tempfile().unwrap();
    temp.as_file_mut()
        .write_all(br#"{"concepts": [{"id": "t1", "metadata": {}}], "associations": []}"#)
        .unwrap();

    // Test open_analytics helper implicitly via run_analytics
    let cmd = csm_duckdb::cli::AnalyticsCommand::Stats(csm_duckdb::cli::StatsArgs {
        input: temp.path().to_path_buf(),
        format: CliOutputFormat::Json,
    });

    csm_duckdb::cli::run_analytics(cmd).await.unwrap();
}
