use std::path::Path;
use tracing::instrument;
use crate::cli::args::{TraverseArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use crate::graph_traversal::TraversalConfig;
use colored::Colorize;
use super::{create_framework, validate_concept_id};

#[instrument(name = "cli_traverse")]
pub async fn run_traverse(
    args: TraverseArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.start)?;
    let framework = create_framework(db_path).await?;

    let config = TraversalConfig {
        max_depth: args.depth,
        min_strength: args.min_strength as f32,
        max_results: 1000,
    };

    let results = framework
        .traverse(&args.start, config)
        .await
        .map_err(|e| CliError::Persistence(format!("traversal failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&results).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            if format != OutputFormat::Quiet {
                println!("{} from {}:", "Traversed".green(), args.start.bold());
                println!("{:<40} {:>6}", "CONCEPT ID", "DEPTH");
                println!("{:-<40} {:->6}", "", "");
            }
            for (id, depth) in results {
                if format == OutputFormat::Quiet {
                    println!("{}", id);
                } else {
                    println!("{:<40} {:>6}", id, depth);
                }
            }
        }
    }
    Ok(())
}
