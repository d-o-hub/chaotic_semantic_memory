use std::path::Path;
use tracing::instrument;
use crate::cli::args::{AssociationsArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use colored::Colorize;
use super::{create_framework, validate_concept_id};

#[instrument(name = "cli_associations")]
pub async fn run_associations(
    args: AssociationsArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.id)?;
    let framework = create_framework(db_path).await?;

    let results = if args.reverse {
        framework.incoming_associations(&args.id).await
    } else {
        framework.get_associations(&args.id).await
    }.map_err(|e| CliError::Persistence(format!("failed to get associations: {e}")))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&results).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            if results.is_empty() {
                if format != OutputFormat::Quiet {
                    println!("No associations found for '{}'", args.id);
                }
            } else {
                if format != OutputFormat::Quiet {
                    let dir = if args.reverse { "Incoming" } else { "Outgoing" };
                    println!("{} associations for {}:", dir.green(), args.id.bold());
                    println!("{:<40} {:>10}", "TARGET/SOURCE", "STRENGTH");
                    println!("{:-<40} {:->10}", "", "");
                }
                for (id, strength) in results {
                    if format == OutputFormat::Quiet {
                        println!("{}", id);
                    } else {
                        println!("{:<40} {:>10.4}", id, strength);
                    }
                }
            }
        }
    }
    Ok(())
}
