use std::path::Path;

use tracing::instrument;

use crate::cli::args::{ExportArgs, ExportFormat, OutputFormat};
use crate::cli::error::{CliError, Result};

use super::{create_framework_with_namespace, print_success, print_warning};

#[instrument(name = "cli_export")]
pub async fn run_export(
    args: ExportArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    let path_str = args.output.to_string_lossy();
    let stats = framework
        .stats()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get stats: {e}")))?;

    if stats.concept_count == 0 {
        print_warning("exporting empty memory state", format);
    } else if matches!(format, OutputFormat::Table) {
        eprintln!(
            "Exporting {} concepts to {}...",
            stats.concept_count,
            args.output.display()
        );
    }

    let result = match args.format {
        ExportFormat::Json => framework.export_json(&path_str).await,
        ExportFormat::Binary => framework.export_binary(&path_str).await,
    };

    match result {
        Ok(()) => {
            print_success(
                &format!(
                    "exported {} concepts to {}",
                    stats.concept_count,
                    args.output.display()
                ),
                format,
            );
            if matches!(format, OutputFormat::Json) {
                println!(
                    "{}",
                    serde_json::json!({
                        "exported": stats.concept_count,
                        "path": args.output.display().to_string()
                    })
                );
            }
        }
        Err(e) => {
            return Err(CliError::Output(format!("export failed: {e}")));
        }
    }

    Ok(())
}
