//! Observability module for CSM (ADR-0072).
//!
//! Provides OTLP gRPC/HTTP export, Prometheus metrics, and structured logging.

use std::net::SocketAddr;
use tracing_subscriber::{EnvFilter, prelude::*};

#[cfg(feature = "otlp")]
pub mod otlp;
#[cfg(feature = "prometheus")]
pub mod prom;

/// Log format for the tracing subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Pretty, human-readable output.
    Pretty,
    /// JSON output for log shipping.
    Json,
    /// Compact output for minimal overhead.
    Compact,
}

/// Configuration for the observability stack.
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Service name for OTLP spans and metrics.
    pub service_name: String,
    /// OTLP gRPC endpoint (e.g., "http://localhost:4317").
    pub otlp_endpoint: Option<String>,
    /// Prometheus scrape bind address (e.g., "127.0.0.1:9090").
    pub prometheus_bind: Option<SocketAddr>,
    /// Log format for stdout.
    pub log_format: LogFormat,
    /// Minimum log level (default: INFO).
    pub log_level: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "csm".to_string(),
            otlp_endpoint: None,
            prometheus_bind: None,
            log_format: LogFormat::Pretty,
            log_level: "info".to_string(),
        }
    }
}

/// Guard to keep the observability stack alive.
pub struct ObservabilityGuard {
    #[cfg(feature = "otlp")]
    _otlp_guard: Option<otlp::OtlpGuard>,
}

/// Initialize the observability stack.
///
/// This should be called once at application startup.
pub fn init(config: ObservabilityConfig) -> crate::error::Result<ObservabilityGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Setup logging layer based on format
    let log_layer = match config.log_format {
        LogFormat::Json => tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_target(false)
            .boxed(),
        LogFormat::Compact => tracing_subscriber::fmt::layer()
            .compact()
            .with_target(false)
            .boxed(),
        LogFormat::Pretty => tracing_subscriber::fmt::layer()
            .pretty()
            .with_target(false)
            .boxed(),
    };

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(log_layer);

    #[cfg(feature = "otlp")]
    let otlp_guard = if let Some(endpoint) = config.otlp_endpoint {
        let (otlp_layer, guard) = otlp::init_otlp(&config.service_name, &endpoint)?;
        let subscriber = registry.with(otlp_layer);
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| crate::error::MemoryError::Config(format!("Failed to set global subscriber: {}", e)))?;
        Some(guard)
    } else {
        tracing::subscriber::set_global_default(registry)
            .map_err(|e| crate::error::MemoryError::Config(format!("Failed to set global subscriber: {}", e)))?;
        None
    };

    #[cfg(not(feature = "otlp"))]
    tracing::subscriber::set_global_default(registry)
        .map_err(|e| crate::error::MemoryError::Config(format!("Failed to set global subscriber: {}", e)))?;

    #[cfg(feature = "prometheus")]
    if let Some(bind_addr) = config.prometheus_bind {
        prom::init_prometheus(bind_addr);
    }

    Ok(ObservabilityGuard {
        #[cfg(feature = "otlp")]
        _otlp_guard: otlp_guard,
    })
}
