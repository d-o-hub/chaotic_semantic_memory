//! Disassociate command for removing concept associations.

use std::path::Path;

use tracing::instrument;

use crate::cli::args::{DisassociateArgs, OutputFormat};
use crate::cli::error::{CliError, Result};

use super::{create_framework, print_success, print_warning, validate_concept_id};

#[instrument(name = "cli_disassociate")]
pub async fn run_disassociate(
    args: DisassociateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.from)?;
    if let Some(ref to) = args.to {
        validate_concept_id(to)?;
    }

    let framework = create_framework(db_path).await?;

    // Check if source concept exists
    let source_exists = framework
        .get_concept(&args.from)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check source concept: {e}")))?
        .is_some();

    if !source_exists {
        return Err(CliError::Input(format!(
            "concept '{}' not found",
            args.from
        )));
    }

    // Check current associations
    let current_associations = framework
        .get_associations(&args.from)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get associations: {e}")))?;

    match &args.to {
        Some(to_id) => {
            // Check if target concept exists
            let target_exists = framework
                .get_concept(to_id)
                .await
                .map_err(|e| CliError::Persistence(format!("failed to check target concept: {e}")))?
                .is_some();

            if !target_exists {
                return Err(CliError::Input(format!("concept '{to_id}' not found")));
            }

            // Check if association exists
            let assoc_exists = current_associations.iter().any(|(id, _)| id == to_id);

            if !assoc_exists {
                return Err(CliError::Input(format!(
                    "no association exists: {} -> {}",
                    args.from, to_id
                )));
            }

            // Remove single association
            framework
                .disassociate(&args.from, to_id)
                .await
                .map_err(|e| {
                    CliError::Persistence(format!(
                        "failed to disassociate '{}' -> '{}': {e}",
                        args.from, to_id
                    ))
                })?;

            match format {
                crate::cli::args::OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "disassociated",
                            "from": args.from,
                            "to": to_id
                        })
                    );
                }
                crate::cli::args::OutputFormat::Table => {
                    print_success(
                        &format!("association removed: {} -> {}", args.from, to_id),
                        format,
                    );
                }
                crate::cli::args::OutputFormat::Quiet => {}
            }
        }
        None => {
            // Clear all associations from source
            if current_associations.is_empty() {
                print_warning(
                    &format!("concept '{}' has no associations to clear", args.from),
                    format,
                );
                return Ok(());
            }

            let count = current_associations.len();

            framework
                .clear_associations(&args.from)
                .await
                .map_err(|e| {
                    CliError::Persistence(format!(
                        "failed to clear associations for '{}': {e}",
                        args.from
                    ))
                })?;

            match format {
                crate::cli::args::OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "cleared",
                            "from": args.from,
                            "count": count
                        })
                    );
                }
                crate::cli::args::OutputFormat::Table => {
                    print_success(
                        &format!("cleared {} associations from '{}'", count, args.from),
                        format,
                    );
                }
                crate::cli::args::OutputFormat::Quiet => {}
            }
        }
    }

    Ok(())
}
