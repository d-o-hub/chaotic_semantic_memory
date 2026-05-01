pub mod associate;
pub mod completions;
pub mod export;
pub mod import;
pub mod index_dir;
pub mod index_jsonl;
pub mod inject;
pub mod mcp;
pub mod probe;
pub mod query;

pub use associate::run_associate;
pub use completions::run_completions;
pub use export::run_export;
pub use import::run_import;
pub use index_dir::run_index_dir;
pub use index_jsonl::run_index_jsonl;
pub use inject::run_inject;
pub use mcp::run_mcp;
pub use probe::run_probe;
pub use query::run_query;

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
    let mut builder = ChaoticSemanticFramework::builder();
    if let Some(path) = db_path {
        builder = builder.with_local_db(path.to_string_lossy());
    } else {
        builder = builder.without_persistence();
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
            "top_k exceeds limit (max 10000, got {})",
            top_k
        )));
    }
    Ok(())
}

fn validate_strength(strength: f64) -> Result<()> {
    if !strength.is_finite() {
        return Err(CliError::Validation(format!(
            "strength must be finite (got {})",
            strength
        )));
    }
    if strength < 0.0 {
        return Err(CliError::Validation(format!(
            "strength must be >= 0 (got {})",
            strength
        )));
    }
    Ok(())
}
