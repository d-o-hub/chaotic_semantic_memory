use clap::{Args, Subcommand};
use std::path::PathBuf;

#[cfg(feature = "parquet")]
use crate::export_parquet::ParquetCompression;

/// SQL Analytics for Chaotic Semantic Memory.
#[derive(Subcommand, Debug, Clone)]
pub enum AnalyticsCommand {
    /// Open a SQL REPL-like prompt.
    Inspect(InspectArgs),
    /// Run a one-shot SQL query and print the result.
    Query(QueryArgs),
    /// Show summary statistics for concepts and benchmarks.
    Stats(StatsArgs),
    /// Export data as a Parquet bundle.
    Export(ExportArgs),
}

#[derive(Args, Debug, Clone)]
pub struct InspectArgs {
    /// Path to a DuckDB database or a CSM export.json file.
    pub input: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// Path to a DuckDB database or a CSM export.json file.
    pub input: PathBuf,
    /// SQL SELECT query to execute.
    pub sql: String,
    /// Output format: table or json.
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
pub struct StatsArgs {
    /// Path to a DuckDB database or a CSM export.json file.
    pub input: PathBuf,
    /// Output format: table or json.
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    /// Path to a DuckDB database or a CSM export.json file.
    pub input: PathBuf,
    /// Output directory for the Parquet bundle.
    #[arg(short, long, default_value = "export_parquet")]
    pub out: PathBuf,
    /// Compression to use: zstd, snappy, or none.
    #[cfg(feature = "parquet")]
    #[arg(long, default_value = "zstd")]
    pub compression: ParquetCompression,
    /// Row group size for Parquet files.
    #[arg(long, default_value = "122880")]
    pub row_group_size: usize,
    /// Partition by column(s), comma-separated.
    #[arg(long)]
    pub partition_by: Option<String>,
}

#[cfg(feature = "cli")]
pub mod export;
#[cfg(feature = "cli")]
pub mod inspect;
#[cfg(feature = "cli")]
pub mod query;
#[cfg(feature = "cli")]
pub mod stats;

use crate::Analytics;
use crate::error::Result;

pub async fn run_analytics(command: AnalyticsCommand) -> Result<()> {
    match command {
        AnalyticsCommand::Inspect(args) => {
            let mut analytics = open_analytics(&args.input)?;
            inspect::run(&mut analytics).await
        }
        AnalyticsCommand::Query(args) => {
            let analytics = open_analytics(&args.input)?;
            query::run(&analytics, &args.sql, &args.format).await
        }
        AnalyticsCommand::Stats(args) => {
            let analytics = open_analytics(&args.input)?;
            stats::run(&analytics, &args.format).await
        }
        AnalyticsCommand::Export(args) => {
            let analytics = open_analytics(&args.input)?;
            export::run(&analytics, args).await
        }
    }
}

fn open_analytics(path: &std::path::Path) -> Result<Analytics> {
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let mut analytics = Analytics::open_in_memory()?;
        analytics.load_export_json(path)?;
        Ok(analytics)
    } else {
        Analytics::open(path)
    }
}
