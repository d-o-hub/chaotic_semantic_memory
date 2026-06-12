//! GraphRAG retrieval command for hybrid similarity + graph search.

use std::path::Path;

use colored::Colorize;
use tracing::instrument;

use crate::cli::args::{OutputFormat, ProbeGraphArgs};
use crate::cli::error::{CliError, Result};
use crate::retrieval::GraphRagConfig;

use super::{create_framework_with_namespace, print_warning};

#[instrument(name = "cli_probe_graph")]
pub async fn run_probe_graph(
    args: ProbeGraphArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let config = GraphRagConfig {
        anchor_top_k: args.anchors,
        max_hops: args.hops,
        min_assoc_strength: args.min_strength,
        similarity_weight: args.similarity_weight,
        graph_weight: args.graph_weight,
        final_top_k: args.top_k,
    };

    let framework: crate::framework::ChaoticSemanticFramework =
        create_framework_with_namespace(db_path, &args.namespace).await?;

    let results = framework
        .probe_text_with_graph(&args.text, config)
        .await
        .map_err(|e| CliError::Persistence(format!("graph-rag retrieval failed: {e}")))?;

    match format {
        OutputFormat::Json => {
            let results_json: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "concept_id": r.id,
                        "score": r.score,
                        "similarity": r.similarity,
                        "anchor_id": r.anchor_id,
                        "hop_distance": r.hop_distance,
                        "assoc_strength": r.assoc_strength
                    })
                })
                .collect();
            let output = serde_json::json!({
                "query": args.text,
                "config": {
                    "anchors": args.anchors,
                    "hops": args.hops,
                    "top_k": args.top_k,
                    "weights": [args.similarity_weight, args.graph_weight]
                },
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
                    &format!("no results for graph-rag query '{}'", args.text),
                    format,
                );
            } else {
                println!(
                    "{} {} GraphRAG results for '{}' (anchors={}, hops={}):",
                    "Found".green(),
                    results.len(),
                    args.text,
                    args.anchors,
                    args.hops
                );
                println!(
                    "{:<40} {:>10} {:>10} {:>8}",
                    "CONCEPT ID", "SCORE", "SIMILARITY", "HOPS"
                );
                println!("{:-<40} {:->10} {:->10} {:->8}", "", "", "", "");
                for r in &results {
                    let score_str = format!("{:.4}", r.score);
                    let colored = if r.score > 0.8 {
                        score_str.green()
                    } else if r.score > 0.5 {
                        score_str.yellow()
                    } else {
                        score_str.normal()
                    };
                    println!(
                        "{:<40} {:>10} {:>10.4} {:>8}",
                        r.id, colored, r.similarity, r.hop_distance
                    );
                }
            }
        }
        OutputFormat::Quiet => {
            for r in &results {
                println!("{}", r.id);
            }
        }
    }

    Ok(())
}
