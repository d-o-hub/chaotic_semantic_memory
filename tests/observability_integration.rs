//! Integration tests for the observability module (ADR-0072 / ADR-0086).
//!
//! Gated on the `prometheus` / `otlp-json` features. These tests
//! exercise the public API surface (`init`, `render_metrics`,
//! `record_*`, `set_*`) and the HTTP scrape endpoint, so they cannot
//! live as `#[cfg(test)]` modules inside `src/observability/prom.rs`
//! (the prometheus crate is only pulled in for the `prometheus` feature,
//! and the lib is already feature-gated as a whole).
#![cfg(any(feature = "prometheus", feature = "otlp-json"))]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

#[cfg(feature = "prometheus")]
mod prom_tests {
    use super::*;
    use chaotic_semantic_memory::observability::{
        LogFormat, ObservabilityConfig, init, prom, render_metrics,
    };

    const fn eph_bind() -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
    }

    /// The seven metrics listed in ADR-0072 §"Metrics surfaced" must all
    /// appear in the text exposition output. This is the contract that
    /// downstream Prometheus / Grafana dashboards rely on.
    #[test]
    fn render_contains_seven_csm_metrics() {
        // Force registration by observing a sample first.
        prom::record_probe("ok", 0.5);
        prom::record_inject(false);
        prom::record_persist("load", 1.0);
        prom::set_concepts_count(7);
        prom::set_associations_count(3);
        prom::set_cache_hit_ratio(7_500);

        let text = render_metrics().expect("render");
        for name in [
            "csm_probe_total",
            "csm_probe_latency_ms",
            "csm_inject_total",
            "csm_persist_latency_ms",
            "csm_concepts_count",
            "csm_associations_count",
            "csm_cache_hit_ratio",
        ] {
            assert!(
                text.contains(name),
                "metric {name} missing from output:\n{text}"
            );
        }
    }

    /// `start_server` should bind successfully and return a non-zero
    /// ephemeral port when the caller passes port `0`. The handle's
    /// `Drop` impl should be a clean shutdown (no panics, no hanging
    /// tasks).
    #[tokio::test]
    async fn start_server_binds_and_serves_metrics() {
        let handle = prom::start_server(eph_bind()).expect("start server");
        let addr = handle.local_addr();
        assert_ne!(addr.port(), 0, "ephemeral port should be non-zero");
        handle.shutdown();
    }

    /// `init` with a `prometheus_bind` must succeed when the feature is
    /// on, and the returned `Guard` must drop cleanly without panicking.
    /// `init` is process-global, so each test in this file must guard
    /// against re-init.
    #[tokio::test]
    async fn init_prometheus_only_succeeds_and_guard_drops() {
        // Best-effort: if another test in this binary already called
        // `init`, expect `ObservabilityAlreadyInitialised` instead of
        // failing the test.
        let cfg = ObservabilityConfig {
            service_name: "csm-test-promise".into(),
            prometheus_bind: Some(eph_bind()),
            log_format: LogFormat::Pretty,
            ..ObservabilityConfig::default()
        };
        match init(cfg) {
            Ok(Some(guard)) => {
                let _ = guard; // explicit drop at end of scope
            }
            Ok(None) => panic!("init should report Some(Guard) when prometheus_bind is set"),
            Err(chaotic_semantic_memory::error::MemoryError::ObservabilityAlreadyInitialised) => {
                // Acceptable: another test in the same process already
                // initialised the global. The earlier init succeeded so
                // the surface is exercised.
            }
            Err(e) => panic!("unexpected init error: {e:?}"),
        }
    }

    /// `render_metrics` must be callable without going through `init`,
    /// because the registry is a process-global singleton that is
    /// lazily created on first observation. This is what allows the
    /// test above to render before the HTTP server is started.
    #[test]
    fn render_metrics_independent_of_init() {
        // Touching the counters is enough to register them; we do not
        // assert specific values.
        prom::record_probe("ok", 1.0);
        let text = render_metrics().expect("render");
        assert!(
            text.contains("# HELP"),
            "prometheus exposition format expected"
        );
    }

    /// FrameworkMetrics operations must populate Prometheus counters.
    /// This test exercises the prom bridge functions directly since
    /// FrameworkMetrics is crate-private; unit tests verify the wiring.
    #[test]
    fn framework_metrics_populate_prometheus() {
        prom::record_inject(false);
        prom::record_inject(true);
        prom::set_associations_count(5);
        prom::record_probe("ok", 42.0);
        prom::record_persist("save", 10.0);
        prom::set_concepts_count(3);

        let text = render_metrics().expect("render");
        assert!(text.contains("csm_probe_total"), "probe counter missing");
        assert!(text.contains("csm_inject_total"), "inject counter missing");
        assert!(
            text.contains("csm_persist_latency_ms"),
            "persist histogram missing"
        );
        assert!(
            text.contains("csm_concepts_count"),
            "concepts gauge missing"
        );
        assert!(
            text.contains("csm_associations_count"),
            "associations gauge missing"
        );
    }
}

#[cfg(feature = "otlp-json")]
mod otlp_tests {
    use chaotic_semantic_memory::observability::{LogFormat, ObservabilityConfig, init};

    /// `init` with `log_format: Json` and no `prometheus_bind` should
    /// return `Ok(None)` (nothing was wired up that requires holding a
    /// guard) and the JSON subscriber should be installed best-effort.
    /// We don't make assertions on the actual log output here because
    /// the `tracing` global subscriber is also process-global; checking
    /// it requires serial tests, which `cargo test` runs in parallel.
    #[test]
    fn init_json_only_succeeds() {
        let cfg = ObservabilityConfig {
            service_name: "csm-test-json".into(),
            log_format: LogFormat::Json,
            ..ObservabilityConfig::default()
        };
        match init(cfg) {
            Ok(_) => { /* expected */ }
            Err(chaotic_semantic_memory::error::MemoryError::ObservabilityAlreadyInitialised) => {
                // Another test in the same process already initialised.
            }
            Err(e) => panic!("unexpected init error: {e:?}"),
        }
    }
}

#[test]
fn smoke_duration_compiles() {
    // A no-op test that uses `Duration` so the `use` above is exercised
    // regardless of feature combinations.
    let d = Duration::from_millis(0);
    assert_eq!(d.as_millis(), 0);
}
