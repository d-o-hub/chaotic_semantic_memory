//! Graph traversal commands for BFS traversal.

// Casts are intentional for CLI output formatting
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::Path;

use colored::Colorize;
use tracing::instrument;

use crate::cli::args::{OutputFormat, TraverseArgs};
use crate::cli::error::{CliError, Result};
use crate::graph_traversal::TraversalConfig;

use super::{create_framework_with_namespace, print_warning, validate_concept_id};

fn validate_min_strength(min_strength: f64) -> Result<()> {
    if !min_strength.is_finite() {
        return Err(CliError::Validation(format!(
            "min_strength must be finite (got {min_strength})"
        )));
    }
    if !(0.0..=1.0).contains(&min_strength) {
        return Err(CliError::Validation(format!(
            "min_strength must be in range [0.0, 1.0] (got {min_strength})"
        )));
    }
    Ok(())
}

#[instrument(name = "cli_traverse")]
pub async fn run_traverse(
    args: TraverseArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.start)?;
    validate_min_strength(args.min_strength)?;

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    // Verify starting concept exists
    let _concept = framework
        .get_concept(&args.start)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.start)))?;

    let config = TraversalConfig {
        max_depth: args.depth,
        min_strength: args.min_strength as f32,
        max_results: 1000, // Use a reasonable default for CLI
    };

    let results = framework
        .traverse(&args.start, config)
        .await
        .map_err(|e| CliError::Persistence(format!("traversal failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            let results_json: Vec<serde_json::Value> = results
                .iter()
                .map(|(id, depth)| {
                    serde_json::json!({
                        "concept_id": id,
                        "depth": depth
                    })
                })
                .collect();
            let output = serde_json::json!({
                "start": args.start,
                "max_depth": args.depth,
                "min_strength": args.min_strength,
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
            if results.is_empty() {
                print_warning(
                    &format!("no reachable concepts from '{}'", args.start),
                    format,
                );
            } else {
                println!(
                    "{} {} concepts reachable from '{}' (max depth: {}):",
                    "Found".green(),
                    results.len(),
                    args.start,
                    args.depth
                );
                println!("{:<40} {:>8}", "CONCEPT ID", "DEPTH");
                println!("{:-<40} {:->8}", "", "");
                for (id, depth) in &results {
                    let depth_str = depth.to_string();
                    let colored = match *depth {
                        0 => depth_str.green(),
                        1 => depth_str.cyan(),
                        2 => depth_str.yellow(),
                        _ => depth_str.normal(),
                    };
                    println!("{id:<40} {colored:>8}");
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &results {
                println!("{id}");
            }
        }
    }

    Ok(())
}
