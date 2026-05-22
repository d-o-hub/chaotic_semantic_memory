//! Rollback command for rolling back a concept to a historical version.

use super::{create_framework_with_namespace, print_success, validate_concept_id};
use crate::cli::args::{OutputFormat, RollbackArgs};
use crate::cli::error::{CliError, Result};
use colored::Colorize;
use std::io::{self, Write};
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_rollback")]
pub async fn run_rollback(
    args: RollbackArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework = create_framework_with_namespace(db_path, &args.namespace).await?;

    // Check if target version exists
    let target = framework
        .get_version(&args.concept_id, args.to)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to fetch version {}: {e}", args.to)))?;

    if target.is_none() {
        return Err(CliError::Input(format!(
            "concept '{}' version {} not found",
            args.concept_id, args.to
        )));
    }

    // Confirmation prompt (skip if --confirm or non-table output)
    if !args.confirm && matches!(format, OutputFormat::Table) {
        eprintln!(
            "{} Roll back concept '{}' to version {}? [y/N]",
            "Confirm:".yellow(),
            args.concept_id,
            args.to
        );
        io::stdout().flush().map_err(CliError::Io)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(CliError::Io)?;

        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            eprintln!("{} Operation cancelled", "Cancelled:".yellow());
            return Ok(());
        }
    }

    // Perform rollback
    let rolled = framework
        .rollback_to_version(&args.concept_id, args.to)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to perform rollback: {e}")))?;

    let msg = format!(
        "Successfully rolled back concept '{}' to version {} (new modified timestamp: {})",
        rolled.id, args.to, rolled.modified_at
    );
    print_success(&msg, format);

    Ok(())
}
