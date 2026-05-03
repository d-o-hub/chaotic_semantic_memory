use super::create_framework;
use crate::cli::args::{MetricsArgs, OutputFormat};
use crate::cli::error::Result;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "cli_metrics")]
pub async fn run_metrics(
    args: MetricsArgs,
    db_path: Option<&Path>,
    format: OutputFormat,
) -> Result<()> {
    let framework = create_framework(db_path).await?;
    let metrics = framework.metrics_snapshot().await;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&metrics).unwrap());
        }
        OutputFormat::Table | OutputFormat::Quiet => {
            println!("Concepts Injected: {}", metrics.concepts_injected_total);
            println!(
                "Associations Created: {}",
                metrics.associations_created_total
            );
            println!("Total Probes: {}", metrics.probes_total);
            println!("Avg Probe Latency: {:.2}ms", metrics.avg_probe_latency_ms);
            println!("Cache Hits: {}", metrics.cache_hits_total);
            println!("Cache Misses: {}", metrics.cache_misses_total);
            println!("Reservoir Steps: {}", metrics.reservoir_steps_total);
        }
    }

    if args.reset {
        framework.reset_metrics().await;
        if format != OutputFormat::Quiet {
            eprintln!("Metrics reset.");
        }
    }

    Ok(())
}
