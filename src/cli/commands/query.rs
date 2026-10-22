//! Text-based similarity query for memory-context integration.
//!
//! Encodes input text and searches for similar concepts.
//! Supports hybrid retrieval combining BM25 keyword matching and HDC semantic search.

// Casts are intentional for CLI output formatting
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::cli::args::{OutputFormat, QueryArgs};
use crate::cli::commands::{
    create_framework_advanced, print_success, print_warning, truncate_preview,
};
use crate::cli::error::{CliError, Result};
use crate::retrieval::hybrid::{HybridMode, HybridResult};

use std::path::Path;

pub async fn run_query(
    args: QueryArgs,
    db_path: Option<&Path>,
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

    // Validate keyword_weight if provided
    if let Some(kw) = args.keyword_weight
        && !(0.0..=1.0).contains(&kw)
    {
        return Err(CliError::Validation(format!(
            "keyword-weight must be between 0.0 and 1.0, got {kw}"
        )));
    }

    // Validate mutual exclusivity
    if args.semantic_only && args.keyword_only {
        return Err(CliError::Validation(
            "cannot use both --semantic-only and --keyword-only".into(),
        ));
    }

    // Determine hybrid mode
    let use_hdc = !args.keyword_only;

    // Load framework with provider only if semantic search is enabled
    let provider_name = if use_hdc {
        args.provider.as_deref()
    } else {
        None
    };

    // Load framework with provider and namespace
    let framework =
        create_framework_advanced(db_path, provider_name, args.code_aware, &args.namespace, 0)
            .await?;

    let mode = if args.semantic_only {
        HybridMode::SemanticOnly
    } else if args.keyword_only {
        HybridMode::KeywordOnly
    } else if let Some(keyword_weight) = args.keyword_weight {
        HybridMode::Custom(keyword_weight as f32)
    } else {
        HybridMode::Auto
    };
    let merged_results = match framework
        .probe_text_with_options(&args.text, args.top_k, mode, args.code_aware)
        .await
        .map_err(|e| CliError::Persistence(format!("query operation failed: {e}")))?
    {
        HybridResult::Success(results) => results,
        HybridResult::Abstained(_) => Vec::new(),
    };

    // Filter by min_score
    let filtered: Vec<_> = merged_results
        .into_iter()
        .filter(|(_, score)| *score >= args.min_score as f32)
        .collect();

    match format {
        OutputFormat::Json => {
            let mut results_json: Vec<serde_json::Value> = Vec::new();

            for (id, score) in &filtered {
                // Get concept metadata if available
                let concept = framework.get_concept(id).await.ok().flatten();
                let mut metadata_json = concept
                    .as_ref()
                    .map(|c| serde_json::to_value(&c.metadata).unwrap_or(serde_json::json!({})))
                    .unwrap_or(serde_json::json!({}));
                if let Some(metadata) = metadata_json.as_object_mut() {
                    metadata.remove("searchable_text");
                }

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

                let display_text = if args.compact {
                    truncate_preview(&text, 200)
                } else {
                    text
                };

                results_json.push(serde_json::json!({
                    "concept_id": id,
                    "score": score,
                    "text": display_text,
                    "path": path,
                    "metadata": metadata_json
                }));
            }

            println!(
                "{}",
                serde_json::to_string(&results_json)
                    .map_err(|e| CliError::Output(format!("failed to serialize results: {e}")))?
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
                    println!("{id:<40} {score:>12.4}");
                }
            }
        }
        OutputFormat::Quiet => {
            for (id, _) in &filtered {
                println!("{id}");
            }
        }
    }

    Ok(())
}
