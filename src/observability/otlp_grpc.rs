//! OTLP gRPC tracing exporter (ADR-0072 / ADR-0086, opt-in via `otlp` feature).
//!
//! Exports OpenTelemetry trace spans over gRPC (tonic/prost/protobuf) to a
//! configurable OTLP endpoint such as an OpenTelemetry Collector. This is the
//! full-fat alternative to the lightweight `otlp-json` log subscriber.
//!
//! # WASM
//!
//! gRPC transport requires `tonic` and `prost`, neither of which compile on
//! `wasm32`. The entire module is gated on `cfg(not(target_arch = "wasm32"))`
//! in addition to the `otlp` Cargo feature.
//!
//! # Usage
//!
//! ```ignore
//! use chaotic_semantic_memory::observability::{ObservabilityConfig, init};
//!
//! let config = ObservabilityConfig {
//!     service_name: "my-service".into(),
//!     otlp_endpoint: Some("http://localhost:4317".into()),
//!     ..ObservabilityConfig::default()
//! };
//! let _guard = init(config)?;
//! // Guard holds the tracer provider; dropping it flushes and shuts down.
//! ```

#![cfg(all(feature = "otlp", not(target_arch = "wasm32")))]

use csm_core::error::{MemoryError, Result};
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Guard that owns the OTLP `TracerProvider`.
///
/// Holding the guard keeps the tracer provider alive. Dropping it flushes
/// any buffered spans and shuts down the provider, ensuring clean export
/// before process exit.
pub struct OtlpGuard {
    provider: Option<TracerProvider>,
}

impl OtlpGuard {
    /// Create an `OtlpGuard` with no provider (for testing purposes).
    ///
    /// The guard will be a no-op: `shutdown()` and `Drop` do nothing when
    /// `provider` is `None`.
    pub fn empty() -> Self {
        Self { provider: None }
    }

    /// Flush remaining spans and shut down the provider.
    ///
    /// Idempotent: subsequent calls after the first are no-ops because
    /// `TracerProvider::shutdown` marks the provider as shut down.
    pub fn shutdown(&mut self) {
        if let Some(provider) = self.provider.take() {
            let _ = provider.shutdown();
        }
    }
}

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Create an OTLP gRPC tracer provider and wire it into `tracing`.
///
/// Uses the **simple** (synchronous) span processor so spans are exported
/// immediately — suitable for low-throughput or development workloads.
/// For production, callers may wish to replace this with a batch processor
/// by constructing the provider directly.
///
/// This function also installs a `tracing_subscriber` layer so that
/// [`tracing::info!`] etc. are exported via OTLP. It uses `try_init()`
/// so it is best-effort: if a subscriber is already installed, the layer
/// is silently skipped.
pub fn install_grpc_tracer(endpoint: &str, service_name: &str) -> Result<OtlpGuard> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| MemoryError::Observability(format!("OTLP gRPC exporter build: {e}")))?;

    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        service_name.to_string(),
    )]);

    let provider = TracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("csm-otlp");

    // Set the global tracer provider so that `opentelemetry::global::tracer()`
    // returns our configured tracer. Other crates that call
    // `global::tracer()` will also export via our OTLP pipeline.
    opentelemetry::global::set_tracer_provider(provider.clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Install as a layer on top of the default Registry. `try_init()`
    // is best-effort: if a subscriber was already set (e.g. by
    // `otlp-json::install_json_subscriber`), the layer is silently skipped.
    let _ = tracing_subscriber::registry().with(otel_layer).try_init();

    Ok(OtlpGuard {
        provider: Some(provider),
    })
}
