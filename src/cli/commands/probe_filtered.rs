use super::{create_framework, validate_concept_id, validate_top_k};
use crate::cli::args::{OutputFormat, ProbeFilteredArgs};
use crate::cli::error::{CliError, Result};
use crate::metadata_filter::MetadataFilter;
use colored::Colorize;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_probe_filtered")]
pub async fn run_probe_filtered(
    args: ProbeFilteredArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;
    validate_top_k(args.top_k)?;

    let filter_val: serde_json::Value = serde_json::from_str(&args.filter)
        .map_err(|e| CliError::Validation(format!("invalid filter JSON: {e}")))?;

    let filter = parse_metadata_filter(filter_val)?;

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

fn parse_metadata_filter(v: serde_json::Value) -> Result<MetadataFilter> {
    // Try native format first
    if let Ok(f) = serde_json::from_value::<MetadataFilter>(v.clone()) {
        return Ok(f);
    }

    // Try documented format: {"key": {"$eq": "val"}}
    if let Some(obj) = v.as_object() {
        if obj.len() == 1 {
            let (key, op_val) = obj.iter().next().unwrap();
            if let Some(op_obj) = op_val.as_object() {
                if op_obj.len() == 1 {
                    let (op, val) = op_obj.iter().next().unwrap();
                    match op.as_str() {
                        "$eq" => return Ok(MetadataFilter::Eq(key.clone(), val.clone())),
                        "$in" => {
                            if let Some(arr) = val.as_array() {
                                return Ok(MetadataFilter::In(key.clone(), arr.clone()));
                            }
                        }
                        "$exists" => {
                            if val.as_bool().unwrap_or(false) {
                                return Ok(MetadataFilter::Exists(key.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Err(CliError::Validation("Unsupported filter format. Use either native enum format or documented object-operator syntax.".to_string()))
}
