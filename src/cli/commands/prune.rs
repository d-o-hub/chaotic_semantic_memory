//! Prune command for removing orphaned associations.

use std::path::Path;
use tracing::instrument;

use crate::cli::args::OutputFormat;
use crate::cli::commands::create_framework;
use crate::cli::commands::print_success;
use crate::cli::error::{CliError, Result};

/// Run the prune command.
#[instrument(name = "cli_prune")]
pub async fn run_prune(db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    let framework = create_framework(db_path).await?;

    let count = framework
        .prune_orphans()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to prune orphans: {e}")))?;

    if matches!(format, OutputFormat::Json) {
        let output = serde_json::json!({
            "status": "success",
            "pruned_count": count,
        });
        println!(
            "{}",
            serde_json::to_string(&output)
                .map_err(|e| CliError::Output(format!("failed to serialize prune results: {e}")))?
        );
    } else {
        print_success(&format!("Pruned {count} orphaned association(s)"), format);
    }

    Ok(())
}
