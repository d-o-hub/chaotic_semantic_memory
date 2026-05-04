//! OTLP gRPC/HTTP exporter wiring (ADR-0072).

use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdktrace};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{Layer, registry::LookupSpan};

/// Guard for OTLP resources.
pub struct OtlpGuard {
    _provider: sdktrace::SdkTracerProvider,
}

/// Initialize OTLP tracer and return a tracing layer and guard.
pub fn init_otlp<S>(
    service_name: &str,
    endpoint: &str,
) -> crate::error::Result<(Box<dyn Layer<S> + Send + Sync>, OtlpGuard)>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a> + Send + Sync
{
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .map_err(|e| crate::error::MemoryError::Config(format!("Failed to create OTLP exporter: {}", e)))?;

    let tracer_provider = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = opentelemetry::trace::TracerProvider::tracer(&tracer_provider, "csm");
    let otlp_layer = OpenTelemetryLayer::new(tracer);

    Ok((Box::new(otlp_layer), OtlpGuard { _provider: tracer_provider }))
}

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        // Shutdown provider to flush spans
        let _ = self._provider.force_flush();
    }
}
