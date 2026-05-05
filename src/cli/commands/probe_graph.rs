use std::path::Path;
use crate::cli::args::{ProbeGraphArgs, OutputFormat};
use crate::cli::error::{CliError, Result};
use super::{create_framework_with_namespace, print_warning};
use crate::retrieval::graph_rag::{GraphRagConfig, GraphRagResult};

pub async fn run_probe_graph(
    args: ProbeGraphArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework = create_framework_with_namespace(db_path, &args.namespace).await?;

    let config = GraphRagConfig {
        anchor_top_k: args.anchors,
        max_hops: args.hops,
        min_assoc_strength: args.min_strength,
        similarity_weight: args.similarity_weight,
        graph_weight: args.graph_weight,
        final_top_k: args.top_k,
    };

    let results: Vec<GraphRagResult> = framework
        .probe_text_with_graph(&args.text, config)
        .await
        .map_err(|e| CliError::Persistence(format!("graph-rag retrieval failed: {e}")))?;

    if results.is_empty() {
        print_warning("No results found.", format);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&results).unwrap());
        }
        OutputFormat::Table => {
            println!("{:<40} {:>10} {:>10} {:>10}", "ID", "SCORE", "SIM", "HOPS");
            println!("{:-<40} {:-<10} {:-<10} {:-<10}", "", "", "", "");
            for res in results {
                println!("{:<40} {:>10.4} {:>10.4} {:>10}", res.id, res.score, res.similarity, res.hop_distance);
            }
        }
        OutputFormat::Quiet => {
            for res in results {
                println!("{}", res.id);
            }
        }
    }

    Ok(())
}
