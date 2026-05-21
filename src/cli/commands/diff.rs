//! Diff command for comparing two concept versions.

use super::{create_framework_with_namespace, validate_concept_id};
use crate::cli::args::{DiffArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use colored::Colorize;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_diff")]
pub async fn run_diff(args: DiffArgs, db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework = create_framework_with_namespace(db_path, &args.namespace).await?;
    let diff = framework
        .diff_versions(&args.concept_id, args.from, args.to)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to calculate version diff: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(&diff)
                    .map_err(|e| CliError::Output(format!("failed to serialize diff: {e}")))?
            );
        }
        OutputFormat::Table => {
            println!(
                "{} Diffing concept '{}' from version {} to {}:",
                "✓".green(),
                args.concept_id,
                args.from,
                args.to
            );
            println!(
                "{} {:.6}",
                "Vector Cosine Distance:".bold(),
                diff.vector_cosine_distance
            );

            if !diff.metadata_added.is_empty() {
                println!("{}", "\nAdded Metadata:".green().bold());
                for (k, v) in &diff.metadata_added {
                    let v_str = serde_json::to_string(v).unwrap_or_default();
                    println!("  + {k}: {v_str}");
                }
            }

            if !diff.metadata_removed.is_empty() {
                println!("{}", "\nRemoved Metadata:".red().bold());
                for (k, v) in &diff.metadata_removed {
                    let v_str = serde_json::to_string(v).unwrap_or_default();
                    println!("  - {k}: {v_str}");
                }
            }

            if !diff.metadata_changed.is_empty() {
                println!("{}", "\nChanged Metadata:".yellow().bold());
                for (k, (v_from, v_to)) in &diff.metadata_changed {
                    let from_str = serde_json::to_string(v_from).unwrap_or_default();
                    let to_str = serde_json::to_string(v_to).unwrap_or_default();
                    println!("  ~ {k}: {from_str} -> {to_str}");
                }
            }

            if diff.metadata_added.is_empty()
                && diff.metadata_removed.is_empty()
                && diff.metadata_changed.is_empty()
            {
                println!("\nNo metadata changes.");
            }
        }
        OutputFormat::Quiet => {
            println!("{}", diff.vector_cosine_distance);
        }
    }

    Ok(())
}
