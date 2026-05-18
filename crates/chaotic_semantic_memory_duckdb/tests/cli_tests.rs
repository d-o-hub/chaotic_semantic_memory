#![cfg(feature = "cli")]

use chaotic_semantic_memory_duckdb::cli::CliOutputFormat;
use chaotic_semantic_memory_duckdb::schema::SCHEMA_DDL;
use duckdb::Connection;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_help_snapshots() {
    use clap::CommandFactory;

    #[derive(clap::Parser)]
    #[command(name = "csm-analytics")]
    struct Cli {
        #[command(subcommand)]
        command: chaotic_semantic_memory_duckdb::cli::AnalyticsCommand,
    }

    let mut cmd = Cli::command();
    let help = cmd.render_help().to_string();
    insta::assert_snapshot!(help);
}

#[tokio::test]
async fn test_stats_command() {
    let temp = NamedTempFile::new().unwrap();
    let conn = Connection::open(temp.path()).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    drop(conn);

    let analytics = chaotic_semantic_memory_duckdb::Analytics::open(temp.path()).unwrap();
    // Just verify it doesn't crash and returns OK
    chaotic_semantic_memory_duckdb::cli::stats::run(&analytics, &CliOutputFormat::Table)
        .await
        .unwrap();
    chaotic_semantic_memory_duckdb::cli::stats::run(&analytics, &CliOutputFormat::Json)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_export_command() {
    let temp_db = NamedTempFile::new().unwrap();
    let conn = Connection::open(temp_db.path()).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    drop(conn);

    let out_dir = tempfile::tempdir().unwrap();

    let cmd = chaotic_semantic_memory_duckdb::cli::AnalyticsCommand::Export(
        chaotic_semantic_memory_duckdb::cli::ExportArgs {
            input: temp_db.path().to_path_buf(),
            out: out_dir.path().to_path_buf(),
            #[cfg(feature = "parquet")]
            compression: chaotic_semantic_memory_duckdb::export_parquet::ParquetCompression::None,
            row_group_size: 1000,
            partition_by: None,
        },
    );

    chaotic_semantic_memory_duckdb::cli::run_analytics(cmd)
        .await
        .unwrap();

    // Verify some files were created
    assert!(out_dir.path().join("concepts.parquet").exists());
}

#[tokio::test]
async fn test_query_command() {
    let temp = NamedTempFile::new().unwrap();
    let conn = Connection::open(temp.path()).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    conn.execute(
        "INSERT INTO concepts (id, namespace) VALUES (?, ?)",
        duckdb::params!["c1", "ns1"],
    )
    .unwrap();
    drop(conn);

    let analytics = chaotic_semantic_memory_duckdb::Analytics::open(temp.path()).unwrap();
    chaotic_semantic_memory_duckdb::cli::query::run(
        &analytics,
        "SELECT * FROM concepts",
        &CliOutputFormat::Table,
    )
    .await
    .unwrap();
    chaotic_semantic_memory_duckdb::cli::query::run(
        &analytics,
        "SELECT * FROM concepts",
        &CliOutputFormat::Json,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_export_json_input() {
    let mut temp = NamedTempFile::new().unwrap();
    temp.as_file_mut()
        .write_all(br#"{"concepts": [{"id": "t1", "metadata": {}}], "associations": []}"#)
        .unwrap();

    // Test open_analytics helper implicitly via run_analytics
    let cmd = chaotic_semantic_memory_duckdb::cli::AnalyticsCommand::Stats(
        chaotic_semantic_memory_duckdb::cli::StatsArgs {
            input: temp.path().to_path_buf(),
            format: CliOutputFormat::Json,
        },
    );

    chaotic_semantic_memory_duckdb::cli::run_analytics(cmd)
        .await
        .unwrap();
}
