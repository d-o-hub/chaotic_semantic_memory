use std::path::Path;
use tracing::instrument;
use crate::cli::args::{ProbeFilteredArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use crate::metadata_filter::MetadataFilter;
use colored::Colorize;
use super::{create_framework, validate_concept_id, validate_top_k};

#[instrument(name = "cli_probe_filtered")]
pub async fn run_probe_filtered(
    args: ProbeFilteredArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;
    validate_top_k(args.top_k)?;

    let filter: MetadataFilter = serde_json::from_str(&args.filter)
        .map_err(|e| CliError::Validation(format!("invalid filter JSON: {e}")))?;

    let framework = create_framework(db_path).await?;
    let concept = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.concept_id)))?;

    let results = framework
        .probe_filtered(&concept.vector, args.top_k, &filter)
        .await
        .map_err(|e| CliError::Persistence(format!("probe filtered failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&results).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            if results.is_empty() {
                if format != OutputFormat::Quiet {
                    println!("No matching concepts found.");
                }
            } else {
                if format != OutputFormat::Quiet {
                    println!("{} filtered results:", "Found".green());
                    println!("{:<40} {:>12}", "CONCEPT ID", "SIMILARITY");
                    println!("{:-<40} {:->12}", "", "");
                }
                for (id, score) in results {
                    if format == OutputFormat::Quiet {
                        println!("{}", id);
                    } else {
                        println!("{:<40} {:>12.4}", id, score);
                    }
                }
            }
        }
    }
    Ok(())
}
