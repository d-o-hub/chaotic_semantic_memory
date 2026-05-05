//! Association commands for linking concepts.

// Casts are intentional for CLI output formatting
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::Path;

use tracing::instrument;

use crate::cli::args::{AssociateArgs, OutputFormat};
use crate::cli::error::{CliError, Result};

use super::{
    create_framework_with_namespace, print_error, print_success, print_warning, validate_concept_id,
    validate_strength,
};

#[instrument(name = "cli_associate")]
pub async fn run_associate(
    args: AssociateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.source_id)?;
    validate_concept_id(&args.target_id)?;
    validate_strength(args.strength)?;

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    let source_exists = framework
        .get_concept(&args.source_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check source concept: {e}")))?
        .is_some();
    if !source_exists {
        print_error(&format!("concept '{}' not found", args.source_id));
        return Err(CliError::Input(format!(
            "concept '{}' not found",
            args.source_id
        )));
    }

    let target_exists = framework
        .get_concept(&args.target_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check target concept: {e}")))?
        .is_some();
    if !target_exists {
        print_error(&format!("concept '{}' not found", args.target_id));
        return Err(CliError::Input(format!(
            "concept '{}' not found",
            args.target_id
        )));
    }

    let is_self_assoc = args.source_id == args.target_id;

    framework
        .associate(&args.source_id, &args.target_id, args.strength as f32)
        .await
        .map_err(|e| {
            CliError::Persistence(format!(
                "failed to associate '{}' -> '{}': {e}",
                args.source_id, args.target_id
            ))
        })?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "created",
                    "source": args.source_id,
                    "target": args.target_id,
                    "strength": args.strength
                })
            );
        }
        OutputFormat::Table => {
            if is_self_assoc {
                print_warning(
                    &format!(
                        "self-association created: {} -> {} (strength: {:.2})",
                        args.source_id, args.target_id, args.strength
                    ),
                    format,
                );
            } else {
                print_success(
                    &format!(
                        "association created: {} -> {} (strength: {:.2})",
                        args.source_id, args.target_id, args.strength
                    ),
                    format,
                );
            }
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}

pub async fn run_associate_batch(
    ns: &str,
    associations: Vec<(String, String, f64)>,
    db_path: Option<&Path>,
    format: OutputFormat,
    continue_on_error: bool,
) -> Result<()> {
    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, ns).await?;

    let mut created = 0usize;
    let mut failed = 0usize;

    for (source_id, target_id, strength) in associations {
        match async {
            validate_concept_id(&source_id)?;
            validate_concept_id(&target_id)?;
            validate_strength(strength)?;

            framework
                .associate(&source_id, &target_id, strength as f32)
                .await
                .map_err(|e| CliError::Persistence(format!("association failed: {e}")))?;
            Ok::<(), CliError>(())
        }
        .await
        {
            Err(e) => {
                failed += 1;
                if !continue_on_error {
                    return Err(CliError::Persistence(format!(
                        "batch failed at {source_id} -> {target_id}: {e}"
                    )));
                }
                if matches!(format, OutputFormat::Table) {
                    print_warning(&format!("skipped {source_id} -> {target_id}: {e}"), format);
                }
            }
            Ok(()) => {
                created += 1;
            }
        }
    }

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({"created": created, "failed": failed})
            );
        }
        OutputFormat::Table => {
            print_success(
                &format!("batch complete: {created} created, {failed} failed"),
                format,
            );
        }
        OutputFormat::Quiet => {}
    }

    Ok(())
}
