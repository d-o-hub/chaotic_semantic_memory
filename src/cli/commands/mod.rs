pub mod associate;
pub mod completions;
pub mod export;
pub mod import;
pub mod inject;
pub mod probe;

pub use associate::run_associate;
pub use completions::run_completions;
pub use export::run_export;
pub use import::run_import;
pub use inject::run_inject;
pub use probe::run_probe;

use crate::cli::args::OutputFormat;
use crate::framework::ChaoticSemanticFramework;
use anyhow::Result;
use colored::Colorize;

pub fn print_success(msg: &str, format: OutputFormat) {
    if matches!(format, OutputFormat::Quiet) {
        return;
    }
    if matches!(format, OutputFormat::Json) {
        println!(r#"{{"status":"success","message":"{}"}}"#, msg);
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
        println!(r#"{{"status":"warning","message":"{}"}}"#, msg);
    } else {
        eprintln!("{} {}", "⚠".yellow(), msg);
    }
}

pub async fn create_framework(db_path: Option<&std::path::Path>) -> Result<ChaoticSemanticFramework> {
    let mut builder = ChaoticSemanticFramework::builder();
    if let Some(path) = db_path {
        builder = builder.with_local_db(path.to_string_lossy());
    } else {
        builder = builder.without_persistence();
    }
    Ok(builder.build().await?)
}

fn validate_concept_id(id: &str) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("concept ID cannot be empty");
    }
    if id.len() > 256 {
        anyhow::bail!("concept ID too long (max 256 bytes, got {})", id.len());
    }
    Ok(())
}

fn validate_top_k(top_k: usize) -> Result<()> {
    if top_k == 0 {
        anyhow::bail!("top_k must be at least 1");
    }
    if top_k > 10_000 {
        anyhow::bail!("top_k exceeds limit (max 10000, got {})", top_k);
    }
    Ok(())
}

fn validate_strength(strength: f64) -> Result<()> {
    if !strength.is_finite() {
        anyhow::bail!("strength must be finite (got {})", strength);
    }
    if strength < 0.0 {
        anyhow::bail!("strength must be >= 0 (got {})", strength);
    }
    Ok(())
}
