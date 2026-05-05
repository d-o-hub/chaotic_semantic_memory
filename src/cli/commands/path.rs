//! Path finding commands for shortest path between concepts.

use std::path::Path;

use colored::Colorize;
use tracing::instrument;

use crate::cli::args::{OutputFormat, PathArgs};
use crate::cli::error::{CliError, Result};

use super::{create_framework_with_namespace, print_warning, validate_concept_id};

#[instrument(name = "cli_path")]
pub async fn run_path(args: PathArgs, db_path: Option<&Path>, format: OutputFormat) -> Result<()> {
    validate_concept_id(&args.from)?;
    validate_concept_id(&args.to)?;

    let framework: crate::framework::ChaoticSemanticFramework = create_framework_with_namespace(db_path, &args.namespace).await?;

    // Verify both concepts exist
    let _from_concept = framework
        .get_concept(&args.from)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get source concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.from)))?;

    let _to_concept = framework
        .get_concept(&args.to)
        .await
        .map_err(|e| CliError::Persistence(format!("failed to get target concept: {e}")))?
        .ok_or_else(|| CliError::Input(format!("concept '{}' not found", args.to)))?;

    let path = if args.weighted {
        framework
            .shortest_path(&args.from, &args.to)
            .await
            .map_err(|e| CliError::Persistence(format!("weighted path search failed: {e}")))?
    } else {
        framework
            .shortest_path_hops(&args.from, &args.to)
            .await
            .map_err(|e| CliError::Persistence(format!("unweighted path search failed: {e}")))?
    };

    match format {
        OutputFormat::Json => {
            let algorithm = if args.weighted {
                "weighted"
            } else {
                "unweighted"
            };
            let output = serde_json::json!({
                "from": args.from,
                "to": args.to,
                "algorithm": algorithm,
                "found": path.is_some(),
                "path": path,
                "hops": path.as_ref().map(|p| p.len() - 1)
            });
            println!(
                "{}",
                serde_json::to_string(&output)
                    .map_err(|e| CliError::Output(format!("failed to serialize results: {e}")))?
            );
        }
        OutputFormat::Table => match &path {
            None => {
                print_warning(
                    &format!("no path found from '{}' to '{}'", args.from, args.to),
                    format,
                );
            }
            Some(nodes) if nodes.len() == 1 => {
                println!(
                    "{} {} {} (same concept)",
                    "Path:".green(),
                    args.from,
                    args.to
                );
            }
            Some(nodes) => {
                let algorithm = if args.weighted {
                    "weighted (Dijkstra)"
                } else {
                    "unweighted (BFS)"
                };
                println!(
                    "{} {} hops from '{}' to '{}' ({}):",
                    "Found".green(),
                    nodes.len() - 1,
                    args.from,
                    args.to,
                    algorithm
                );
                for (i, node) in nodes.iter().enumerate() {
                    if i == 0 {
                        println!("  {} {}", "START".cyan(), node);
                    } else if i == nodes.len() - 1 {
                        println!("  {} {}", "END".green(), node);
                    } else {
                        println!("  {:>5} {}", format!("→{i}").yellow(), node);
                    }
                }
            }
        },
        OutputFormat::Quiet => {
            if let Some(nodes) = &path {
                for node in nodes {
                    println!("{node}");
                }
            }
        }
    }

    Ok(())
}
