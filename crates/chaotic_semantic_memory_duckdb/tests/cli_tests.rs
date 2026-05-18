#![cfg(feature = "cli")]

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
    let mut temp = NamedTempFile::new().unwrap();
    let conn = Connection::open(temp.path()).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    drop(conn);

    let analytics = chaotic_semantic_memory_duckdb::Analytics::open(temp.path()).unwrap();
    // Just verify it doesn't crash and returns OK
    chaotic_semantic_memory_duckdb::cli::stats::run(&analytics, "table")
        .await
        .unwrap();
    chaotic_semantic_memory_duckdb::cli::stats::run(&analytics, "json")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_query_command() {
    let mut temp = NamedTempFile::new().unwrap();
    let conn = Connection::open(temp.path()).unwrap();
    conn.execute_batch(SCHEMA_DDL).unwrap();
    conn.execute(
        "INSERT INTO concepts (id, namespace) VALUES (?, ?)",
        duckdb::params!["c1", "ns1"],
    )
    .unwrap();
    drop(conn);

    let analytics = chaotic_semantic_memory_duckdb::Analytics::open(temp.path()).unwrap();
    chaotic_semantic_memory_duckdb::cli::query::run(&analytics, "SELECT * FROM concepts", "table")
        .await
        .unwrap();
    chaotic_semantic_memory_duckdb::cli::query::run(&analytics, "SELECT * FROM concepts", "json")
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
            format: "json".to_string(),
        },
    );

    chaotic_semantic_memory_duckdb::cli::run_analytics(cmd)
        .await
        .unwrap();
}
