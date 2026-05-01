use std::path::Path;
use tracing::instrument;
use crate::cli::args::{PathArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use crate::graph_traversal::TraversalConfig;
use colored::Colorize;
use super::{create_framework, validate_concept_id};

#[instrument(name = "cli_path")]
pub async fn run_path(
    args: PathArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.from)?;
    validate_concept_id(&args.to)?;
    let framework = create_framework(db_path).await?;

    let path = if args.weighted {
        framework.shortest_path(&args.from, &args.to).await
    } else {
        let singularity = framework.singularity();
        let sing = singularity.read().await;
        sing.shortest_path_hops(&args.from, &args.to, &TraversalConfig::default())
    }.map_err(|e| CliError::Persistence(format!("shortest path calculation failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&path).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            match path {
                Some(nodes) => {
                    if format == OutputFormat::Quiet {
                        for node in nodes {
                            println!("{}", node);
                        }
                    } else {
                        println!("{} path found ({} hops):", "Shortest".green(), nodes.len() - 1);
                        println!("{}", nodes.join(" -> ".cyan().to_string().as_str()));
                    }
                }
                None => {
                    if format != OutputFormat::Quiet {
                        println!("No path found between '{}' and '{}'", args.from, args.to);
                    }
                }
            }
        }
    }
    Ok(())
}
