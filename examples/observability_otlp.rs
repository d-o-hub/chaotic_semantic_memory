//! End-to-end observability example for `chaotic_semantic_memory` (ADR-0072).
//!
//! Demonstrates wiring up the Prometheus `/metrics` HTTP endpoint and the
//! JSON log subscriber, running a small probe workload, and printing the
//! rendered metrics to stdout. The Prometheus server continues to serve
//! the live registry while the process is alive — connect a scraper to
//! `127.0.0.1:9090/metrics` to inspect.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example observability_otlp --features prometheus,otlp-json
//! ```
//!
//! Then in another terminal:
//!
//! ```bash
//! curl -s http://127.0.0.1:9090/metrics | grep csm_
//! ```

use std::time::Duration;

use chaotic_semantic_memory::observability::{
    self, LogFormat, ObservabilityConfig, init, prom, render_metrics,
};
use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Bring up observability. Both features are opt-in; the binary
    //    must be built with `--features prometheus,otlp-json` to enable
    //    this surface.
    let cfg = ObservabilityConfig {
        service_name: "csm-observability-example".into(),
        // Standard Prometheus port. Change to `0` for ephemeral binding
        // in tests.
        prometheus_bind: Some("127.0.0.1:9090".parse()?),
        log_format: LogFormat::Json,
        ..ObservabilityConfig::default()
    };
    let _guard = init(cfg)?;

    // 2. Boot an in-memory framework (no persistence — this is a
    //    end-to-end smoke test, not a durability test).
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // 3. Inject a small corpus and run a probe. The framework's tracing
    //    spans (see `#[instrument]` in src/singularity.rs) emit log
    //    events that the JSON subscriber formats to stdout.
    for i in 0..16 {
        let id = format!("concept-{i}");
        let vector = HVec10240::random();
        prom::record_inject(false);
        framework.inject_concept(id, vector).await?;
    }
    let query = HVec10240::random();
    let start = std::time::Instant::now();
    let _hits = framework.probe(query, 5).await?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    prom::record_probe("ok", elapsed_ms);
    prom::set_concepts_count(16);

    // 4. Give Prometheus a moment to record the request, then render the
    //    registry ourselves so the user can see what the scraper will get.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let snapshot = render_metrics()?;
    let csm_lines: Vec<&str> = snapshot
        .lines()
        .filter(|l| l.starts_with("csm_") && !l.starts_with("csm_probe_latency_ms_bucket"))
        .take(20)
        .collect();
    println!("# rendered CSM metrics (subset):");
    for line in csm_lines {
        println!("{line}");
    }
    println!(
        "# prometheus server still listening on 127.0.0.1:9090 — \
         curl http://127.0.0.1:9090/metrics"
    );

    // 5. The `_guard` keeps the Prometheus server alive for as long as
    //    this scope is held. The framework's `Drop` impl handles
    //    shutdown on the way out.
    observability::render_metrics()?;
    Ok(())
}
