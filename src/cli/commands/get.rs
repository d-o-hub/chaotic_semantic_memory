use std::path::Path;
use tracing::instrument;
use crate::cli::args::{GetArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use super::{create_framework, validate_concept_id};

#[instrument(name = "cli_get")]
pub async fn run_get(
    args: GetArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.id)?;
    let framework = create_framework(db_path).await?;

    let concept = framework
        .get_concept(&args.id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.id)))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&concept).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            println!("ID: {}", concept.id);
            println!("Created: {}", concept.created_at);
            println!("Modified: {}", concept.modified_at);
            if let Some(expires) = concept.expires_at {
                println!("Expires: {}", expires);
            }
            println!("Metadata: {}", serde_json::to_string(&concept.metadata).unwrap());
            if !concept.canonical_concept_ids.is_empty() {
                println!("Canonical Links: {:?}", concept.canonical_concept_ids);
            }
            if format != OutputFormat::Quiet {
                println!("Vector: [{} dims]", concept.vector.data.len());
            }
        }
    }
    Ok(())
}
