//! Compact command for reclaiming database space.

use std::path::Path;
use tracing::instrument;

use crate::cli::args::OutputFormat;
use crate::cli::commands::create_framework;
use crate::cli::commands::print_success;
use crate::cli::error::{CliError, Result};

/// Run the compact command.
#[instrument(name = "cli_compact")]
pub async fn run_compact(db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    let framework = create_framework(db_path).await?;

    framework
        .compact()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to compact database: {e}")))?;

    print_success("Database compacted successfully", format);

    Ok(())
}
