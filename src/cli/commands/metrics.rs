//! Metrics command for performance metrics.

use std::path::Path;

use tracing::instrument;

use crate::cli::args::OutputFormat;
use crate::cli::error::{CliError, Result};

use super::create_framework;

/// Run the metrics command.
#[instrument(name = "cli_metrics")]
pub async fn run_metrics(db_path: Option<&Path>, format: OutputFormat, reset: bool) -> Result<()> {
    let framework: crate::framework::ChaoticSemanticFramework = create_framework(db_path).await?;

    if reset {
        // Note: Metrics reset is not currently supported at the framework level.
        // This is a placeholder for future implementation.
        return Err(CliError::Validation(
            "metrics reset is not yet implemented".to_string(),
        ));
    }

    let snapshot = framework.metrics_snapshot().await;

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "status": "ok",
                "metrics": {
                    "concepts_injected_total": snapshot.concepts_injected_total,
                    "associations_created_total": snapshot.associations_created_total,
                    "probes_total": snapshot.probes_total,
                    "avg_probe_latency_ms": snapshot.avg_probe_latency_ms,
                    "cache_hits_total": snapshot.cache_hits_total,
                    "cache_misses_total": snapshot.cache_misses_total,
                    "cache_evictions_total": snapshot.cache_evictions_total,
                    "reservoir_steps_total": snapshot.reservoir_steps_total,
                    "avg_reservoir_step_latency_us": snapshot.avg_reservoir_step_latency_us,
                    "reservoir_nodes_active": snapshot.reservoir_nodes_active,
                }
            });
            println!(
                "{}",
                serde_json::to_string(&output)
                    .map_err(|e| CliError::Output(format!("failed to serialize metrics: {e}")))?
            );
        }
        OutputFormat::Table => {
            println!("Framework Metrics:");
            println!(
                "  Concepts injected:    {}",
                snapshot.concepts_injected_total
            );
            println!(
                "  Associations created: {}",
                snapshot.associations_created_total
            );
            println!("  Probes executed:      {}", snapshot.probes_total);
            println!(
                "  Avg probe latency:    {:.2} ms",
                snapshot.avg_probe_latency_ms
            );
            println!();
            println!("Cache Metrics:");
            println!("  Hits:       {}", snapshot.cache_hits_total);
            println!("  Misses:     {}", snapshot.cache_misses_total);
            println!("  Evictions:  {}", snapshot.cache_evictions_total);
            println!();
            println!("Reservoir Metrics:");
            println!("  Steps:          {}", snapshot.reservoir_steps_total);
            println!(
                "  Avg step latency: {:.2} us",
                snapshot.avg_reservoir_step_latency_us
            );
            println!("  Active nodes:   {}", snapshot.reservoir_nodes_active);
        }
        OutputFormat::Quiet => {
            // Output minimal key metrics on separate lines
            println!("concepts_injected:{}", snapshot.concepts_injected_total);
            println!("probes:{}", snapshot.probes_total);
            println!("cache_hits:{}", snapshot.cache_hits_total);
        }
    }

    Ok(())
}
