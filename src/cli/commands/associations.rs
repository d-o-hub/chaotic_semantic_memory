//! Association query commands for listing concept associations.

// Casts are intentional for CLI output formatting
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::Path;

use colored::Colorize;
use tracing::instrument;

use crate::cli::args::{AssociationsArgs, OutputFormat};
use crate::cli::error::{CliError, Result};

use super::{create_framework, create_framework_with_namespace, print_warning, validate_concept_id};

#[instrument(name = "cli_associations")]
pub async fn run_associations(
    args: AssociationsArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework: crate::framework::ChaoticSemanticFramework = create_framework_with_namespace(db_path, &args.namespace).await?;

    // Verify concept exists
    let _concept = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.concept_id)))?;

    let associations = if args.reverse {
        framework
            .incoming_associations(&args.concept_id)
            .await
            .map_err(|e| {
                CliError::Persistence(format!("failed to get incoming associations: {e}"))
            })?
    } else {
        framework
            .get_associations(&args.concept_id)
            .await
            .map_err(|e| CliError::Persistence(format!("failed to get associations: {e}")))?
    };

    match format {
        OutputFormat::Json => {
            let direction = if args.reverse { "incoming" } else { "outbound" };
            let results_json: Vec<serde_json::Value> = associations
                .iter()
                .map(|(id, strength)| {
                    serde_json::json!({
                        "concept_id": id,
                        "strength": strength
                    })
                })
                .collect();
            let output = serde_json::json!({
                "query_id": args.concept_id,
                "direction": direction,
                "count": results_json.len(),
                "results": results_json
            });
            println!(
                "{}",
                serde_json::to_string(&output)
                    .map_err(|e| CliError::Output(format!("failed to serialize results: {e}")))?
            );
        }
        OutputFormat::Table => {
            let direction = if args.reverse { "incoming" } else { "outbound" };
            if associations.is_empty() {
                print_warning(
                    &format!("no {} associations for '{}'", direction, args.concept_id),
                    format,
                );
            } else {
                println!(
                    "{} {} {} associations for '{}':",
                    "Found".green(),
                    associations.len(),
                    direction,
                    args.concept_id
                );
                println!("{:<40} {:>12}", "CONCEPT ID", "STRENGTH");
                println!("{:-<40} {:->12}", "", "");
                for (id, strength) in &associations {
                    let strength_str = format!("{strength:.4}");
                    let colored = if *strength > 0.8 {
                        strength_str.green()
                    } else if *strength > 0.5 {
                        strength_str.yellow()
                    } else {
                        strength_str.normal()
                    };
                    println!("{id:<40} {colored:>12}");
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &associations {
                println!("{id}");
            }
        }
    }

    Ok(())
}
