use super::{create_framework, print_success, validate_concept_id};
use crate::cli::args::{DisassociateArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_disassociate")]
pub async fn run_disassociate(
    args: DisassociateArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.from)?;
    let framework = create_framework(db_path).await?;

    if let Some(to) = args.to {
        validate_concept_id(&to)?;
        framework
            .disassociate(&args.from, &to)
            .await
            .map_err(|e| CliError::Persistence(format!("failed to disassociate: {e}")))?;
        print_success(
            &format!("association {} -> {} removed", args.from, to),
            format,
        );
    } else {
        framework
            .clear_associations(&args.from)
            .await
            .map_err(|e| CliError::Persistence(format!("failed to clear associations: {e}")))?;
        print_success(
            &format!("all associations from '{}' cleared", args.from),
            format,
        );
    }

    Ok(())
}
