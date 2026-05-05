#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Stats command for framework statistics.

// Cast is intentional for CLI output formatting

use std::path::Path;

use tracing::instrument;

use crate::cli::args::OutputFormat;
use crate::cli::error::{CliError, Result};

use super::create_framework;

/// Run the stats command.
#[instrument(name = "cli_stats")]
pub async fn run_stats(db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    let framework = create_framework(db_path).await?;

    let stats = framework
        .stats()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get stats: {e}")))?;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "ok",
                "concept_count": stats.concept_count,
                "db_size_bytes": stats.db_size_bytes,
            });
            println!(
                "{}",
                serde_json::to_string(&output)
                    .map_err(|e| CliError::Output(format!("failed to serialize stats: {e}")))?
            );
        }
        OutputFormat::Table => {
            println!("Concept count: {}", stats.concept_count);
            if let Some(db_size) = stats.db_size_bytes {
                println!(
                    "Database size: {} bytes ({:.2} MB)",
                    db_size,
                    db_size as f64 / 1_048_576.0
                );
            } else {
                println!("Database size: N/A (persistence disabled)");
            }
        }
        OutputFormat::Quiet => {
            println!("{}", stats.concept_count);
        }
    }

    Ok(())
}
