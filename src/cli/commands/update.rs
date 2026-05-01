use super::{create_framework, print_success, validate_concept_id};
use crate::cli::args::{OutputFormat, UpdateArgs};
use crate::cli::error::{CliError, Result};
use crate::encoder::TextEncoder;
use std::collections::HashMap;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_update")]
pub async fn run_update(
    args: UpdateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.id)?;
    let framework = create_framework(db_path).await?;

    if let Some(text) = args.vector_from_text {
        let encoder = TextEncoder::new();
        let vector = encoder.encode(&text);
        framework
            .update_concept_vector(&args.id, vector)
            .await
            .map_err(|e| CliError::Persistence(format!("failed to update vector: {e}")))?;
    }

    if let Some(metadata_json) = args.metadata {
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_json)
            .map_err(|e| CliError::Validation(format!("invalid metadata JSON: {e}")))?;
        framework
            .update_concept_metadata(&args.id, metadata)
            .await
            .map_err(|e| CliError::Persistence(format!("failed to update metadata: {e}")))?;
    }

    print_success(&format!("concept '{}' updated", args.id), format);
    Ok(())
}
