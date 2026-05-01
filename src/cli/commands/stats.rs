use std::path::Path;
use tracing::instrument;
use crate::cli::args::{StatsArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use super::create_framework;

#[instrument(name = "cli_stats")]
pub async fn run_stats(
    _args: StatsArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework = create_framework(db_path).await?;
    let stats = framework
        .stats()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get stats: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            println!("Concepts: {}", stats.concept_count);
            if let Some(size) = stats.db_size_bytes {
                println!("Database Size: {} bytes", size);
            }
        }
    }
    Ok(())
}
