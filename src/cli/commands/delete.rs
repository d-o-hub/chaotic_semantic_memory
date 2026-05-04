//! Delete command for removing concepts from memory.

use std::io::{self, Write};
use std::path::Path;

use tracing::instrument;

use crate::cli::args::{DeleteArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use colored::Colorize;

use super::{create_framework, create_framework_with_namespace, print_success, validate_concept_id};

#[instrument(name = "cli_delete")]
pub async fn run_delete(
    args: DeleteArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework: crate::framework::ChaoticSemanticFramework = create_framework_with_namespace(db_path, &args.namespace).await?;

    // Check if concept exists before deletion
    let existing = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check concept: {e}")))?;

    if existing.is_none() {
        return Err(CliError::Input(format!(
            "concept '{}' not found",
            args.concept_id
        )));
    }

    // Confirmation prompt (skip if --force or non-table output)
    if !args.force && matches!(format, crate::cli::args::OutputFormat::Table) {
        eprintln!(
            "{} Delete concept '{}'? [y/N]",
            "Confirm:".yellow(),
            args.concept_id
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

    // Perform deletion
    framework
        .delete_concept(&args.concept_id)
        .await
        .map_err(|e| {
            CliError::Persistence(format!(
                "failed to delete concept '{}': {e}",
                args.concept_id
            ))
        })?;

    match format {
        crate::cli::args::OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "deleted",
                    "concept_id": args.concept_id
                })
            );
        }
        crate::cli::args::OutputFormat::Table => {
            print_success(&format!("concept '{}' deleted", args.concept_id), format);
        }
        crate::cli::args::OutputFormat::Quiet => {}
    }

    Ok(())
}
