use chaotic_semantic_memory_duckdb::cli::{AnalyticsCommand, run_analytics};
use clap::Parser;

#[derive(Parser)]
#[command(name = "csm-analytics")]
#[command(about = "SQL Analytics for Chaotic Semantic Memory", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: AnalyticsCommand,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run_analytics(cli.command)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}
