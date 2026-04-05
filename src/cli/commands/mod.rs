pub mod associate;
pub mod completions;
pub mod export;
pub mod import;
pub mod index_dir;
pub mod index_jsonl;
pub mod inject;
pub mod probe;
pub mod query;

pub use associate::run_associate;
pub use completions::run_completions;
pub use export::run_export;
pub use import::run_import;
pub use index_dir::run_index_dir;
pub use index_jsonl::run_index_jsonl;
pub use inject::run_inject;
pub use probe::run_probe;
pub use query::run_query;

use crate::cli::args::OutputFormat;
use crate::cli::error::{CliError, Result};
use crate::cli::git_local::{ensure_git_local_dir, resolve_git_local_path};
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

/// Resolves the database path based on CLI arguments.
///
/// Priority order:
/// 1. Explicit --database path (if provided)
/// 2. Explicit --index-path (if provided with --git-local)
/// 3. Git-local storage (.git/memory-index/csm.db) if in git repo and no --database
/// 4. None (in-memory mode) if not in git repo and no --database
///
/// Returns a tuple of (resolved_path, should_use_git_local)
fn resolve_database_path(
    database: Option<&std::path::Path>,
    git_local: bool,
    index_path: Option<&std::path::Path>,
) -> Result<Option<std::path::PathBuf>> {
    // Case 1: Explicit --database path provided
    if let Some(db_path) = database {
        return Ok(Some(db_path.to_path_buf()));
    }

    // Case 2: --index-path override with --git-local
    if let Some(custom_path) = index_path {
        if !git_local {
            return Err(CliError::Config(
                "--index-path requires --git-local to be specified".to_string(),
            ));
        }
        return Ok(Some(custom_path.to_path_buf()));
    }

    // Case 3: --git-local explicitly requested
    if git_local {
        let path = resolve_git_local_path().ok_or_else(|| {
            CliError::Config(
                "--git-local specified but not in a git repository. \
                 Run this command inside a git repo or use --database to specify a path."
                    .to_string(),
            )
        })?;
        ensure_git_local_dir(&path).map_err(|e| {
            CliError::Config(format!("Failed to create git-local directory: {}", e))
        })?;
        return Ok(Some(path));
    }

    // Case 4: Default - try git-local storage
    if let Some(path) = resolve_git_local_path() {
        // Found a git repo, use git-local storage by default
        ensure_git_local_dir(&path).map_err(|e| {
            CliError::Config(format!("Failed to create git-local directory: {}", e))
        })?;
        return Ok(Some(path));
    }

    // Case 5: Not in git repo and no database specified - use in-memory mode
    Ok(None)
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

/// Create framework with full argument handling including git-local support.
///
/// This is the preferred entry point for CLI commands that need database access.
pub async fn create_framework_with_args(
    database: Option<&std::path::Path>,
    git_local: bool,
    index_path: Option<&std::path::Path>,
) -> Result<ChaoticSemanticFramework> {
    let resolved_path = resolve_database_path(database, git_local, index_path)?;
    create_framework(resolved_path.as_deref()).await
}

fn validate_concept_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(CliError::Validation("concept ID cannot be empty".into()));
    }
    if id.len() > 256 {
        return Err(CliError::Validation(format!(
            "concept ID too long (max 256 bytes, got {})",
            id.len()
        )));
    }
    Ok(())
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
