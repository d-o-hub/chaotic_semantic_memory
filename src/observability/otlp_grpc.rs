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

use csm_core_lib::error::{MemoryError, Result};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider};
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::OtlpBatchConfig;

/// Guard that owns the OTLP `TracerProvider`.
///
/// Holding the guard keeps the tracer provider alive. Explicit shutdown flushes
/// spans and stops the batch processor within the configured timeout.
pub struct OtlpGuard {
    provider: Option<SdkTracerProvider>,
    shutdown_timeout: Duration,
}

impl OtlpGuard {
    /// Create an `OtlpGuard` with no provider (for testing purposes).
    ///
    /// The guard will be a no-op: `shutdown()` and `Drop` do nothing when
    /// `provider` is `None`.
    pub fn empty() -> Self {
        Self {
            provider: None,
            shutdown_timeout: Duration::from_secs(5),
        }
    }

    /// Flush queued spans; each remote export request is bounded by
    /// `max_export_timeout`. This synchronous SDK method has no overall
    /// deadline, so several queued batches can take longer than that bound.
    ///
    /// # Errors
    ///
    /// Returns an exporter error, including a request timeout.
    pub fn force_flush(&self) -> Result<()> {
        if let Some(provider) = &self.provider {
            provider.force_flush().map_err(|error| {
                MemoryError::Observability(format!("OTLP force flush: {error}"))
            })?;
        }
        Ok(())
    }

    /// Shut down the provider once, waiting at most `shutdown_timeout`.
    ///
    /// The batch processor drains queued spans as part of shutdown.
    ///
    /// # Errors
    ///
    /// Returns exporter shutdown failures or a timeout.
    pub fn shutdown(&mut self) -> Result<()> {
        let Some(provider) = self.provider.take() else {
            return Ok(());
        };
        provider
            .shutdown_with_timeout(self.shutdown_timeout)
            .map_err(|error| MemoryError::Observability(format!("OTLP shutdown: {error}")))
    }
}

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            tracing::warn!(%error, "OTLP tracer shutdown failed");
        }
    }
}

/// Create an OTLP gRPC tracer provider and wire it into `tracing`.
///
/// Uses a bounded batch span processor configured by `batch_config`; its queue
/// and export batch size cap buffered telemetry. `OtlpGuard` exposes bounded
/// `force_flush` and `shutdown` operations and reports their failures.
///
/// This function also installs a `tracing_subscriber` layer so that
/// [`tracing::info!`] etc. are exported via OTLP. It uses `try_init()`
/// so it is best-effort: if a subscriber is already installed, the layer
/// is silently skipped.
pub fn install_grpc_tracer(
    endpoint: &str,
    service_name: &str,
    batch_config: &OtlpBatchConfig,
) -> Result<OtlpGuard> {
    batch_config.validate()?;
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        // The SDK's `BatchConfigBuilder::with_max_export_timeout` only exists
        // behind the experimental async-runtime processor, so the export
        // request timeout is bound on the exporter's own channel instead.
        .with_timeout(batch_config.max_export_timeout)
        .build()
        .map_err(|e| MemoryError::Observability(format!("OTLP gRPC exporter build: {e}")))?;

    let processor = BatchSpanProcessor::builder(exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_queue_size(batch_config.max_queue_size)
                .with_max_export_batch_size(batch_config.max_export_batch_size)
                .with_scheduled_delay(batch_config.scheduled_delay)
                .build(),
        )
        .build();
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    let provider = SdkTracerProvider::builder()
        .with_span_processor(processor)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("csm-otlp");

    // Global state is the propagator used by MCP's HTTP middleware to turn
    // incoming W3C traceparent headers into parent spans.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    opentelemetry::global::set_tracer_provider(provider.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let _ = tracing_subscriber::registry().with(otel_layer).try_init();

    Ok(OtlpGuard {
        provider: Some(provider),
        shutdown_timeout: batch_config.shutdown_timeout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_configuration_rejects_invalid_bounds() {
        let mut config = OtlpBatchConfig::default();
        assert!(config.validate().is_ok());

        config.max_queue_size = 0;
        assert!(config.validate().is_err());

        config = OtlpBatchConfig::default();
        config.max_export_batch_size = config.max_queue_size + 1;
        assert!(config.validate().is_err());

        config = OtlpBatchConfig::default();
        config.scheduled_delay = Duration::ZERO;
        assert!(config.validate().is_err());

        config = OtlpBatchConfig::default();
        config.max_export_timeout = Duration::ZERO;
        assert!(config.validate().is_err());

        config = OtlpBatchConfig::default();
        config.shutdown_timeout = Duration::ZERO;
        assert!(config.validate().is_err());
    }

    #[test]
    fn empty_guard_flush_and_shutdown_are_idempotent() {
        let mut guard = OtlpGuard::empty();
        assert!(guard.force_flush().is_ok());
        assert!(guard.shutdown().is_ok());
        assert!(guard.shutdown().is_ok());
    }
}
