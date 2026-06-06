pub mod associate;
pub mod associations;
pub mod completions;
pub mod delete;
pub mod diff;
pub mod disassociate;
pub mod export;
pub mod get;
pub mod history;
pub mod import;
pub mod index_dir;
pub mod index_jsonl;
pub mod inject;
pub mod metrics;
pub mod path;
pub mod probe;
pub mod probe_filtered;
pub mod probe_graph;
pub mod query;
pub mod rollback;
pub mod stats;
pub mod traverse;
pub mod update;
pub mod watch;

pub use associate::run_associate;
pub use associations::run_associations;
pub use completions::run_completions;
pub use delete::run_delete;
pub use diff::run_diff;
pub use disassociate::run_disassociate;
pub use export::run_export;
pub use get::run_get;
pub use history::run_history;
pub use import::run_import;
pub use index_dir::run_index_dir;
pub use index_jsonl::run_index_jsonl;
pub use inject::run_inject;
pub use metrics::run_metrics;
pub use path::run_path;
pub use probe::run_probe;
pub use probe_filtered::run_probe_filtered;
pub use probe_graph::run_probe_graph;
pub use query::run_query;
pub use rollback::run_rollback;
pub use stats::run_stats;
pub use traverse::run_traverse;
pub use update::run_update;
pub use watch::run_watch;

use crate::cli::args::OutputFormat;
use crate::cli::error::{CliError, Result};
use crate::framework::ChaoticSemanticFramework;
use colored::Colorize;

pub fn print_success(msg: &str, format: OutputFormat) {
    if matches!(format, OutputFormat::Quiet) {
        return;
    }
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::json!({"status": "success", "message": msg})
        );
    } else {
        eprintln!("{} {}", "✓".green(), msg);
    }
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

pub fn print_warning(msg: &str, format: OutputFormat) {
    if matches!(format, OutputFormat::Quiet) {
        return;
    }
    if matches!(format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::json!({"status": "warning", "message": msg})
        );
    } else {
        eprintln!("{} {}", "⚠".yellow(), msg);
    }
}

/// Truncate a string to `max_chars` characters, appending "..." if truncated.
///
/// Uses `char_indices` to avoid panicking on multi-byte UTF-8 boundaries.
pub fn truncate_preview(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}...", &s[..idx]),
        None => s.to_string(),
    }
}

pub async fn create_framework(
    db_path: Option<&std::path::Path>,
) -> Result<ChaoticSemanticFramework> {
    create_framework_advanced(db_path, None, false, "_default").await
}

pub async fn create_framework_with_namespace(
    db_path: Option<&std::path::Path>,
    ns: &str,
) -> Result<ChaoticSemanticFramework> {
    create_framework_advanced(db_path, None, false, ns).await
}

pub async fn create_framework_with_provider(
    db_path: Option<&std::path::Path>,
    provider_name: Option<&str>,
) -> Result<ChaoticSemanticFramework> {
    create_framework_advanced(db_path, provider_name, false, "_default").await
}

pub async fn create_framework_advanced(
    db_path: Option<&std::path::Path>,
    provider_name: Option<&str>,
    code_aware: bool,
    ns: &str,
) -> Result<ChaoticSemanticFramework> {
    let mut builder = ChaoticSemanticFramework::builder();
    if let Some(path) = db_path {
        builder = builder.with_local_db(path.to_string_lossy());
    } else {
        builder = builder.without_persistence();
    }
    builder = builder.with_namespace(ns);

    if let Some(name) = provider_name {
        let provider = crate::embedding::get_provider(name)
            .map_err(|e| CliError::Config(format!("failed to load embedding provider: {e}")))?;

        // If provider is HDC and code-aware is requested, apply config
        if provider.name() == "hdc-text" && code_aware {
            builder =
                builder.with_embedding_provider(crate::embedding::HdcTextProvider::with_config(
                    csm_core::encoder::TextEncoderConfig {
                        ngram_size: Some(3),
                        code_aware: true,
                        ..Default::default()
                    },
                ));
        } else {
            builder = builder.with_embedding_provider_arc(provider);
        }
    } else if code_aware {
        // Default HDC provider with code-aware config
        builder = builder.with_embedding_provider(crate::embedding::HdcTextProvider::with_config(
            csm_core::encoder::TextEncoderConfig {
                ngram_size: Some(3),
                code_aware: true,
                ..Default::default()
            },
        ));
    }

    builder
        .build()
        .await
        .map_err(|e| CliError::Persistence(format!("failed to initialize framework: {e}")))
}

fn validate_concept_id(id: &str) -> Result<()> {
    ChaoticSemanticFramework::validate_concept_id(id)
        .map_err(|e| CliError::Validation(e.to_string()))
}

fn validate_top_k(top_k: usize) -> Result<()> {
    if top_k == 0 {
        return Err(CliError::Validation("top_k must be at least 1".into()));
    }
    if top_k > 10_000 {
        return Err(CliError::Validation(format!(
            "top_k exceeds limit (max 10000, got {top_k})"
        )));
    }
    Ok(())
}

fn validate_strength(strength: f64) -> Result<()> {
    if !strength.is_finite() {
        return Err(CliError::Validation(format!(
            "strength must be finite (got {strength})"
        )));
    }
    if !(0.0..=1.0).contains(&strength) {
        return Err(CliError::Validation(format!(
            "strength must be in [0.0, 1.0] (got {strength})"
        )));
    }
    Ok(())
}
