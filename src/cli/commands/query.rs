//! Text-based similarity query for memory-context integration.
//!
//! Encodes input text and searches for similar concepts.

use crate::cli::args::{OutputFormat, QueryArgs};
use crate::cli::commands::{create_framework_with_args, print_success, print_warning};
use crate::cli::error::{CliError, Result};
use crate::encoder::TextEncoder;

use std::path::Path;

pub async fn run_query(
    args: QueryArgs,
    database: Option<&Path>,
    git_local: bool,
    index_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    // Validate min_score range
    if args.min_score < 0.0 || args.min_score > 1.0 {
        return Err(CliError::Validation(format!(
            "min-score must be between 0.0 and 1.0, got {}",
            args.min_score
        )));
    }

    // Validate top_k
    if args.top_k == 0 {
        return Err(CliError::Validation("top-k must be at least 1".into()));
    }

    let framework = create_framework_with_args(database, git_local, index_path).await?;

    // Create encoder based on code_aware flag
    let encoder = if args.code_aware {
        TextEncoder::new_code_aware()
    } else {
        TextEncoder::new()
    };

    // Encode the query text
    let query_vector = encoder.encode(&args.text);

    // Search for similar concepts
    let results = framework
        .probe(query_vector, args.top_k)
        .await
        .map_err(|e| CliError::Persistence(format!("query operation failed: {}", e)))?;

    // Filter by min_score
    let filtered: Vec<_> = results
        .into_iter()
        .filter(|(_, score)| *score >= args.min_score as f32)
        .collect();

    match format {
        OutputFormat::Json => {
            let mut results_json: Vec<serde_json::Value> = Vec::new();

            for (id, score) in &filtered {
                // Get concept metadata if available
                let concept = framework.get_concept(id).await.ok().flatten();
                let metadata_json = concept
                    .as_ref()
                    .map(|c| serde_json::to_value(&c.metadata).unwrap_or(serde_json::json!({})))
                    .unwrap_or(serde_json::json!({}));

                let text = metadata_json
                    .get("text_preview")
                    .or_else(|| metadata_json.get("content_preview"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let path = metadata_json
                    .get("source")
                    .or_else(|| metadata_json.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let display_text = if args.compact && text.len() > 200 {
                    format!("{}...", &text[..200])
                } else {
                    text
                };

                results_json.push(serde_json::json!({
                    "score": score,
                    "text": display_text,
                    "path": path,
                    "metadata": metadata_json
                }));
            }

            println!(
                "{}",
                serde_json::to_string(&results_json)
                    .map_err(|e| CliError::Output(format!("failed to serialize results: {}", e)))?
            );
        }
        OutputFormat::Table => {
            if filtered.is_empty() {
                print_warning("no similar concepts found", format);
            } else {
                print_success(&format!("Found {} results", filtered.len()), format);
                println!("{:<40} {:>12}", "CONCEPT ID", "SCORE");
                println!("{:-<40} {:-<12}", "", "");
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
