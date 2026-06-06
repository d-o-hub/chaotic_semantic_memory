//! Observability exports for `chaotic_semantic_memory` (ADR-0072).
//!
//! Opt-in modules behind Cargo features:
//! - `prometheus` — `/metrics` HTTP scrape endpoint + counters/histograms.
//! - `otlp-json` — JSON-structured tracing for log shippers (lightweight
//!   alternative to full OTLP gRPC; no `opentelemetry-otlp` dependency).
//!
//! All features are non-default. The crate's baseline tracing behaviour
//! (stderr `tracing_subscriber::FmtSubscriber`) is unchanged unless the
//! caller calls [`init`].
#![cfg(any(feature = "prometheus", feature = "otlp-json"))]
#![allow(clippy::module_name_repetitions)]

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use csm_core::error::{MemoryError, Result};

#[cfg(feature = "prometheus")]
pub mod prom;

#[cfg(feature = "otlp-json")]
pub mod otlp;

/// Tracks whether [`init`] has been called in this process.
static INITIALISED: AtomicBool = AtomicBool::new(false);

/// Log formatting selection for [`init`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Plain text to stderr (`tracing_subscriber::fmt` default).
    #[default]
    Pretty,
    /// Single-line JSON to stdout (log-shipping friendly).
    Json,
    /// Newline-delimited JSON to stdout (NDJSON; OTLP-friendly).
    Ndjson,
}

/// Observability configuration.
///
/// All fields are optional. `init` returns `Ok(None)` when nothing was
/// actually enabled (no `prometheus_bind` and `log_format == Pretty`),
/// so tests can call it unconditionally.
#[derive(Debug, Clone, Default)]
pub struct ObservabilityConfig {
    /// Service name reported in logs and metrics.
    pub service_name: String,
    /// OTLP endpoint placeholder (e.g. `http://localhost:4317`).
    /// Reserved for future gRPC export; today it only drives the
    /// `service.name` resource attribute and the JSON log banner.
    pub otlp_endpoint: Option<String>,
    /// Bind address for the Prometheus `/metrics` HTTP server.
    /// Requires the `prometheus` feature.
    pub prometheus_bind: Option<SocketAddr>,
    /// Log line format.
    pub log_format: LogFormat,
}

/// Guard returned by [`init`].
///
/// Holding the guard keeps the Prometheus server task running. Drop it to
/// gracefully shut down the scrape endpoint.
#[must_use = "dropping the guard stops the prometheus scrape server"]
pub struct Guard {
    #[cfg(feature = "prometheus")]
    prom_handle: Option<prom::PromServerHandle>,
    // Marker that ensures the type has a non-zero size in feature
    // combinations that do not enable `prometheus` (no `prom_handle`).
    _priv: (),
}

impl Drop for Guard {
    fn drop(&mut self) {
        #[cfg(feature = "prometheus")]
        if let Some(handle) = self.prom_handle.take() {
            handle.shutdown();
        }
    }
}

/// Initialise observability based on `config`.
///
/// `init` is idempotent in a single process: a second call returns
/// [`MemoryError::ObservabilityAlreadyInitialised`] so callers can detect
/// double-init instead of silently doubling the log output.
///
/// Returns `Ok(None)` when nothing was actually enabled — useful for tests
/// that want to call this unconditionally.
pub fn init(config: ObservabilityConfig) -> Result<Option<Guard>> {
    if INITIALISED.swap(true, Ordering::SeqCst) {
        return Err(MemoryError::ObservabilityAlreadyInitialised);
    }

    // Pre-flight: if the caller asked for a feature we don't have, fail
    // early before any state changes are observable.
    #[cfg(not(feature = "prometheus"))]
    if config.prometheus_bind.is_some() {
        INITIALISED.store(false, Ordering::SeqCst);
        return Err(MemoryError::ObservabilityFeatureDisabled {
            feature: "prometheus",
        });
    }
    #[cfg(not(feature = "otlp-json"))]
    if matches!(config.log_format, LogFormat::Json | LogFormat::Ndjson) {
        INITIALISED.store(false, Ordering::SeqCst);
        return Err(MemoryError::ObservabilityFeatureDisabled {
            feature: "otlp-json",
        });
    }

    #[cfg(feature = "otlp-json")]
    {
        if matches!(config.log_format, LogFormat::Json | LogFormat::Ndjson) {
            // NDJSON is single-line JSON to stdout — same wire shape as
            // `Json` for our purposes (one event per line, parseable by
            // Fluent Bit/Vector/Promtail). Kept as a separate variant for
            // caller intent.
            otlp::install_json_subscriber();
        }
    }

    // Decide what we actually brought up.
    let have_prom_server = cfg!(feature = "prometheus") && config.prometheus_bind.is_some();

    #[cfg(feature = "prometheus")]
    let prom_handle = if let Some(bind) = config.prometheus_bind {
        Some(prom::start_server(bind)?)
    } else {
        None
    };
    #[cfg(not(feature = "prometheus"))]
    let _ = have_prom_server;

    tracing::info!(
        service = %config.service_name,
        otlp_endpoint = ?config.otlp_endpoint,
        prometheus_bind = ?config.prometheus_bind,
        "observability initialised"
    );

    if !have_prom_server && matches!(config.log_format, LogFormat::Pretty) {
        // Nothing was actually wired up; release the init flag so the next
        // call can still attempt to bring the stack up.
        INITIALISED.store(false, Ordering::SeqCst);
        return Ok(None);
    }

    Ok(Some(Guard {
        #[cfg(feature = "prometheus")]
        prom_handle,
        _priv: (),
    }))
}

/// Encode the current Prometheus registry into the text exposition format.
///
/// Requires the `prometheus` feature. Returns `Err` if the feature is
/// disabled or the registry cannot be encoded.
pub fn render_metrics() -> Result<String> {
    #[cfg(feature = "prometheus")]
    {
        prom::render()
    }
    #[cfg(not(feature = "prometheus"))]
    {
        Err(MemoryError::ObservabilityFeatureDisabled {
            feature: "prometheus",
        })
    }
}
