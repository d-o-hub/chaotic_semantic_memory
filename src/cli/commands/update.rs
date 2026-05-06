//! Update command for modifying concept vector or metadata.

use std::collections::HashMap;
use std::path::Path;

use tracing::instrument;

use crate::cli::args::{OutputFormat, UpdateArgs};
use crate::cli::error::{CliError, Result};
use crate::encoder::TextEncoder;

use super::{create_framework_with_namespace, print_success, validate_concept_id};

#[instrument(name = "cli_update")]
pub async fn run_update(
    args: UpdateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;

    // Must provide at least one update option
    if args.vector_from_text.is_none() && args.metadata.is_none() {
        return Err(CliError::Input(
            "must provide --vector-from-text or --metadata to update".into(),
        ));
    }

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    // Check if concept exists
    let existing = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to check concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.concept_id)))?;

    // Update vector if requested
    if let Some(ref text) = args.vector_from_text {
        let encoder = if args.code_aware {
            TextEncoder::new_code_aware()
        } else {
            TextEncoder::new()
        };
        let new_vector = encoder.encode(text);

        framework
            .update_concept_vector(&args.concept_id, new_vector)
            .await
            .map_err(|e| {
                CliError::Persistence(format!(
                    "failed to update vector for '{}': {e}",
                    args.concept_id
                ))
            })?;
    }

    // Update metadata if requested
    if let Some(ref metadata_json) = args.metadata {
        let new_metadata: HashMap<String, serde_json::Value> = serde_json::from_str(metadata_json)
            .map_err(|e| CliError::Validation(format!("invalid metadata JSON: {e}")))?;

        let final_metadata = if args.replace_metadata {
            new_metadata
        } else {
            // Merge with existing metadata
            let mut merged = existing.metadata.clone();
            for (key, value) in new_metadata {
                merged.insert(key, value);
            }
            merged
        };

        framework
            .update_concept_metadata(&args.concept_id, final_metadata)
            .await
            .map_err(|e| {
                CliError::Persistence(format!(
                    "failed to update metadata for '{}': {e}",
                    args.concept_id
                ))
            })?;
    }

    match format {
        crate::cli::args::OutputFormat::Json => {
            let updates: Vec<&str> = [
                args.vector_from_text.as_ref().map(|_| "vector"),
                args.metadata.as_ref().map(|_| "metadata"),
            ]
            .into_iter()
            .flatten()
            .collect();

            println!(
                "{}",
                serde_json::json!({
                    "status": "updated",
                    "concept_id": args.concept_id,
                    "updates": updates
                })
            );
        }
        crate::cli::args::OutputFormat::Table => {
            let mut updates = Vec::new();
            if args.vector_from_text.is_some() {
                updates.push("vector");
            }
            if args.metadata.is_some() {
                if args.replace_metadata {
                    updates.push("metadata (replaced)");
                } else {
                    updates.push("metadata (merged)");
                }
            }
            print_success(
                &format!(
                    "concept '{}' updated: {}",
                    args.concept_id,
                    updates.join(", ")
                ),
                format,
            );
        }
        crate::cli::args::OutputFormat::Quiet => {}
    }

    Ok(())
}
