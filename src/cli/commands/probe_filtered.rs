//! Filtered probe commands for similarity search with metadata filtering.

// Casts are intentional for CLI output formatting

use std::path::Path;

use colored::Colorize;
use tracing::instrument;

use crate::cli::args::{OutputFormat, ProbeFilteredArgs};
use crate::cli::error::{CliError, Result};
use crate::metadata_filter::MetadataFilter;

use super::{create_framework, print_warning, validate_concept_id, validate_top_k};

#[instrument(name = "cli_probe_filtered")]
pub async fn run_probe_filtered(
    args: ProbeFilteredArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;
    validate_top_k(args.top_k)?;

    // Parse metadata filter from JSON
    let filter: MetadataFilter = serde_json::from_str(&args.filter)
        .map_err(|e| CliError::Validation(format!("invalid filter JSON: {e}")))?;

    let framework = create_framework(db_path).await?;

    // Get concept to use its vector as query
    let concept = framework
        .get_concept(&args.concept_id)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.concept_id)))?;

    let results = framework
        .probe_filtered(&concept.vector, args.top_k, &filter)
        .await
        .map_err(|e| CliError::Persistence(format!("filtered probe failed: {e}")))?;

    // Filter out the query concept itself
    let filtered: Vec<_> = results
        .into_iter()
        .filter(|(id, _)| id != &args.concept_id)
        .collect();

    match format {
        OutputFormat::Json => {
            let results_json: Vec<serde_json::Value> = filtered
                .iter()
                .map(|(id, score)| {
                    serde_json::json!({
                        "concept_id": id,
                        "similarity": score
                    })
                })
                .collect();
            let output = serde_json::json!({
                "query_id": args.concept_id,
                "filter": args.filter,
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
            if filtered.is_empty() {
                print_warning(
                    &format!(
                        "no similar concepts found matching filter for '{}'",
                        args.concept_id
                    ),
                    format,
                );
            } else {
                println!(
                    "{} {} filtered results for '{}':",
                    "Found".green(),
                    filtered.len(),
                    args.concept_id
                );
                println!("Filter: {}", args.filter);
                println!("{:<40} {:>12}", "CONCEPT ID", "SIMILARITY");
                println!("{:-<40} {:->12}", "", "");
                for (id, score) in &filtered {
                    let score_str = format!("{score:.4}");
                    let colored = if *score > 0.8 {
                        score_str.green()
                    } else if *score > 0.5 {
                        score_str.yellow()
                    } else {
                        score_str.normal()
                    };
                    println!("{id:<40} {colored:>12}");
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &filtered {
                println!("{id}");
            }
        }
    }

    Ok(())
}
