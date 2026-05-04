//! Example demonstrating OTLP and Prometheus observability (ADR-0072).
//!
//! Run with:
//! cargo run --example observability_otlp --features otlp,prometheus,cli

use chaotic_semantic_memory::observability::{self, LogFormat, ObservabilityConfig};
use chaotic_semantic_memory::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configure observability
    let config = ObservabilityConfig {
        service_name: "csm-example".to_string(),
        otlp_endpoint: Some("http://localhost:4317".to_string()),
        prometheus_bind: Some("127.0.0.1:9090".parse::<SocketAddr>().unwrap()),
        log_format: LogFormat::Pretty,
        log_level: "info".to_string(),
    };

    // 2. Initialize (returns a guard that must be kept alive)
    println!("Initializing observability...");
    println!("OTLP endpoint: http://localhost:4317");
    println!("Prometheus metrics: http://127.0.0.1:9090/metrics");

    let _guard = observability::init(config)?;

    // 3. Build framework
    let framework = FrameworkBuilder::new().build().await?;

    // 4. Perform some operations to generate spans and metrics
    println!("Injecting concepts...");
    for i in 0..10 {
        let id = format!("concept-{}", i);
        let vector = HVec10240::random();
        framework.inject_concept(id, vector).await?;
    }

    println!("Creating associations...");
    framework.associate("concept-0", "concept-1", 0.8).await?;
    framework.associate("concept-1", "concept-2", 0.5).await?;

    println!("Running probes...");
    for _ in 0..5 {
        let query = HVec10240::random();
        let _results = framework.probe(query, 5).await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // 5. Trigger metrics update
    println!("Updating metrics snapshot...");
    let _snapshot = framework.metrics_snapshot().await;

    println!("Done! Observability data has been emitted.");
    println!("Keep this process running to scrape metrics, or Ctrl+C to exit.");

    tokio::time::sleep(Duration::from_secs(5)).await;

    Ok(())
}
