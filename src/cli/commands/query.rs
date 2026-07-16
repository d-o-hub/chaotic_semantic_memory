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
use crate::retrieval::bm25::Bm25Index;
use crate::retrieval::hybrid::{HybridResult, compute_weights, merge_results};

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
    if let Some(kw) = args.keyword_weight {
        if !(0.0..=1.0).contains(&kw) {
            return Err(CliError::Validation(format!(
                "keyword-weight must be between 0.0 and 1.0, got {kw}"
            )));
        }
    }

    // Validate mutual exclusivity
    if args.semantic_only && args.keyword_only {
        return Err(CliError::Validation(
            "cannot use both --semantic-only and --keyword-only".into(),
        ));
    }

    // Determine hybrid mode
    let use_bm25 = !args.semantic_only;
    let use_hdc = !args.keyword_only;

    // Load framework with provider only if semantic search is enabled
    let provider_name = if use_hdc {
        args.provider.as_deref()
    } else {
        None
    };

    // Load framework with provider and namespace
    let framework =
        create_framework_advanced(db_path, provider_name, args.code_aware, &args.namespace).await?;

    // Tokenize query for BM25
    let query_tokens = tokenize_query(&args.text, args.code_aware);

    // Collect results from both search methods
    let hdc_results = if use_hdc {
        // Search for similar concepts using framework's probe_text (which uses configured provider)
        match framework
            .probe_text(&args.text, args.top_k)
            .await
            .map_err(|e| CliError::Persistence(format!("query operation failed: {e}")))?
        {
            HybridResult::Success(results) => Some(results),
            HybridResult::Abstained(_) => None,
        }
    } else {
        None
    };

    let bm25_results = if use_bm25 {
        // ADR-0094 / Wave 32: short-circuit BM25 only when HDC also produced no
        // hits this query (never for --keyword-only). Absence rows come from
        // semantic abstentions; inject clears them (invalidate_absence_short_circuit).
        #[cfg(feature = "persistence")]
        let skip_bm25 = {
            use crate::retrieval::bm25::{DEFAULT_ABSENCE_MIN_ATTEMPTS, is_known_absent};
            // Do not skip the only retrieval path; require concurrent HDC empty.
            let hdc_also_empty = use_hdc && hdc_results.is_none();
            if hdc_also_empty {
                if let Some(store) = framework.persistence_store() {
                    is_known_absent(&args.text, store.as_ref(), DEFAULT_ABSENCE_MIN_ATTEMPTS).await
                } else {
                    false
                }
            } else {
                false
            }
        };
        #[cfg(not(feature = "persistence"))]
        let skip_bm25 = false;

        if skip_bm25 {
            tracing::debug!(
                query = %args.text,
                "skipping BM25: known absence short-circuit (HDC also empty)"
            );
            None
        } else {
            // Build BM25 index from concepts
            let bm25_index = build_bm25_index(&framework).await?;
            if bm25_index.is_empty() {
                None
            } else {
                Some(bm25_index.search(&query_tokens, args.top_k))
            }
        }
    } else {
        None
    };

    // Merge or use single source
    let merged_results = match (bm25_results, hdc_results) {
        (Some(bm25), Some(hdc)) => {
            // Compute weights
            let weights = if let Some(kw) = args.keyword_weight {
                (kw as f32, (1.0 - kw) as f32)
            } else {
                compute_weights(query_tokens.len())
            };

            merge_results(&bm25, &hdc, weights)
        }
        (Some(bm25), None) => bm25,
        (None, Some(hdc)) => hdc,
        (None, None) => Vec::new(),
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

                let display_text = if args.compact {
                    truncate_preview(&text, 200)
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

/// Tokenize query text for BM25.
fn tokenize_query(text: &str, code_aware: bool) -> Vec<String> {
    let processed = text.to_lowercase();

    if code_aware {
        // Use code-aware tokenization (split on separators)
        tokenize_code(&processed)
    } else {
        // Simple whitespace tokenization
        processed
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
}

/// Tokenize code-aware text (same logic as TextEncoder::tokenize_code).
fn tokenize_code(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();

    for word in text.split_whitespace() {
        let parts = split_on_separators(word);
        tokens.extend(parts);
    }

    tokens
}

/// Split a word on code separators.
fn split_on_separators(word: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = word.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for `::` (double colon)
        if i + 1 < chars.len() && chars[i] == ':' && chars[i + 1] == ':' {
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            i += 2;
            continue;
        }

        // Check for single-char separators: `_`, `-`, `.`, `/`
        let c = chars[i];
        if c == '_' || c == '-' || c == '.' || c == '/' {
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }
            i += 1;
            continue;
        }

        current.push(c);
        i += 1;
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Build BM25 index from concepts in the framework.
async fn build_bm25_index(
    framework: &crate::framework::ChaoticSemanticFramework,
) -> Result<Bm25Index> {
    // Collect concepts with lock, build index without lock
    let concepts = {
        let singularity = framework.singularity();
        let sing = singularity.read().await;
        let ns = framework.namespace().await;
        sing.all_concepts(&ns)
    };

    let mut index = Bm25Index::new();
    for concept in concepts {
        // Extract tokens from text_preview or content_preview
        let text = concept
            .metadata
            .get("text_preview")
            .or_else(|| concept.metadata.get("content_preview"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let tokens: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if !tokens.is_empty() {
            index.add_document(&concept.id, &tokens);
        }
    }

    Ok(index)
}
