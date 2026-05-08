//! Get command for retrieving concept details.

use std::path::Path;

use tracing::instrument;

use crate::cli::args::{GetArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use crate::hyperdim::Hypervector;
use colored::Colorize;

use super::{create_framework_with_namespace, validate_concept_id};

#[instrument(name = "cli_get")]
pub async fn run_get(args: GetArgs, db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    let concept = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.concept_id)))?;

    // Get associations for this concept
    let associations = framework
        .get_associations(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get associations: {e}")))?;

    match format {
        crate::cli::args::OutputFormat::Json => {
            let assoc_json: Vec<serde_json::Value> = associations
                .iter()
                .map(|(id, strength)| {
                    serde_json::json!({
                        "target_id": id,
                        "strength": strength
                    })
                })
                .collect();

            println!(
                "{}",
                serde_json::json!({
                    "concept_id": concept.id,
                    "vector_preview": format!("{:?}", &concept.vector.to_bytes()[0..8]),
                    "metadata": concept.metadata,
                    "created_at": concept.created_at,
                    "modified_at": concept.modified_at,
                    "expires_at": concept.expires_at,
                    "associations": assoc_json,
                    "canonical_concept_ids": concept.canonical_concept_ids
                })
            );
        }
        crate::cli::args::OutputFormat::Table => {
            println!("{} {}", "Concept:".green(), concept.id);
            println!(
                "{} {}",
                "Created:".green(),
                format_timestamp(concept.created_at)
            );
            println!(
                "{} {}",
                "Modified:".green(),
                format_timestamp(concept.modified_at)
            );
            if let Some(exp) = concept.expires_at {
                println!("{} {}", "Expires:".green(), format_timestamp(exp));
            }

            // Metadata display
            if !concept.metadata.is_empty() {
                println!("{}", "Metadata:".green());
                for (key, value) in &concept.metadata {
                    let value_str = if value.is_string() {
                        value.as_str().unwrap_or("").to_string()
                    } else {
                        serde_json::to_string(value).unwrap_or_default()
                    };
                    println!("  {key}: {value_str}");
                }
            }

            // Associations display
            if !associations.is_empty() {
                println!(
                    "{} {} associations",
                    "Associations:".green(),
                    associations.len()
                );
                for (target_id, strength) in &associations {
                    println!("  -> {target_id} (strength: {strength:.2})");
                }
            }

            // Canonical concept links
            if !concept.canonical_concept_ids.is_empty() {
                println!(
                    "{} {}",
                    "Canonical links:".green(),
                    concept.canonical_concept_ids.join(", ")
                );
            }
        }
        crate::cli::args::OutputFormat::Quiet => {
            println!("{}", concept.id);
        }
    }

    Ok(())
}

/// Format Unix timestamp as human-readable string.
fn format_timestamp(ts: u64) -> String {
    // Simple ISO-like format without chrono dependency
    let secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    let days = ts / 86400;
    if days > 0 {
        format!("{days}d {hours:02}:{mins:02}:{secs:02}")
    } else {
        format!("{hours:02}:{mins:02}:{secs:02}")
    }
}
