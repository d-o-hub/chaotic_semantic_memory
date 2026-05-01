use std::path::Path;
use tracing::instrument;
use crate::cli::args::{DeleteArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use super::{create_framework, print_success, validate_concept_id};

#[instrument(name = "cli_delete")]
pub async fn run_delete(
    args: DeleteArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.id)?;
    let framework = create_framework(db_path).await?;

    framework
        .delete_concept(&args.id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to delete concept: {e}")))?;

    print_success(&format!("concept '{}' deleted", args.id), format);
    Ok(())
}
