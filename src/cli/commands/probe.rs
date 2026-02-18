use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::args::{OutputFormat, ProbeArgs};
use crate::hyperdim::HVec10240;

use super::{create_framework, print_success, print_warning, validate_concept_id, validate_top_k};

pub async fn run_probe(
    args: ProbeArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_concept_id(&args.concept_id)?;
    validate_top_k(args.top_k)?;

    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let concept = framework
        .get_concept(&args.concept_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("concept '{}' not found", args.concept_id))?;

    let results = framework
        .probe(concept.vector, args.top_k)
        .await
        .context("probe operation failed")?;

    let filtered: Vec<_> = results
        .into_iter()
        .filter(|(id, _)| id != &args.concept_id)
        .filter(|(_, score)| args.threshold.is_none_or(|t| *score >= t as f32))
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
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "query_id": args.concept_id,
                    "count": results_json.len(),
                    "results": results_json
                }))
                .unwrap()
            );
        }
        OutputFormat::Table => {
            if filtered.is_empty() {
                print_warning("no similar concepts found", format);
            } else {
                println!("{} {} results:", "Found".green(), filtered.len());
                println!("{:<40} {:>12}", "CONCEPT ID", "SIMILARITY");
                println!("{:-<40} {:->12}", "", "");
                for (id, score) in &filtered {
                    let score_str = format!("{:.4}", score);
                    let colored = if *score > 0.8 {
                        score_str.green()
                    } else if *score > 0.5 {
                        score_str.yellow()
                    } else {
                        score_str.normal()
                    };
                    println!("{:<40} {:>12}", id, colored);
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &filtered {
                println!("{}", id);
            }
        }
    }

    Ok(())
}

pub async fn run_probe_with_vector(
    query_vector: HVec10240,
    top_k: usize,
    threshold: Option<f64>,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    validate_top_k(top_k)?;

    let framework = create_framework(db_path)
        .await
        .context("failed to initialize framework")?;

    let results = framework
        .probe(query_vector, top_k)
        .await
        .context("probe operation failed")?;

    let filtered: Vec<_> = results
        .into_iter()
        .filter(|(_, score)| threshold.is_none_or(|t| *score >= t as f32))
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
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "count": results_json.len(),
                    "results": results_json
                }))
                .unwrap()
            );
        }
        OutputFormat::Table => {
            if filtered.is_empty() {
                print_warning("no similar concepts found", format);
            } else {
                print_success(&format!("Found {} results", filtered.len()), format);
                println!("{:<40} {:>12}", "CONCEPT ID", "SIMILARITY");
                println!("{:-<40} {:->12}", "", "");
                for (id, score) in &filtered {
                    println!("{:<40} {:>12.4}", id, score);
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &filtered {
                println!("{}", id);
            }
        }
    }

    Ok(())
}
