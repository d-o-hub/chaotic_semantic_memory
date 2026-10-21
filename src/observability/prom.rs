//! Prometheus `/metrics` HTTP endpoint and metric registry (ADR-0072).
//!
//! Provides the seven metrics listed in ADR-0072 plus a minimal
//! `hyper`-based HTTP server that serves the text exposition format on
//! `GET /metrics`. The whole module is gated on the `prometheus` feature
//! because it pulls in `prometheus`, `hyper`, `hyper-util`, and
//! `http-body-util` (none of which are useful without the server).

#![allow(clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::OnceLock;

use prometheus::{
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    Registry, TextEncoder,
};
use tokio::sync::oneshot;

use csm_core_lib::error::{MemoryError, Result};

/// Global registry — created once per process.
fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(Registry::new)
}

/// Global handle to the typed metric vectors.
struct Metrics {
    probe_total: IntCounterVec,
    probe_latency_ms: Histogram,
    inject_total: IntCounterVec,
    persist_latency_ms: HistogramVec,
    import_failures: IntCounterVec,
    stale_snapshot_rebuilds: IntCounterVec,
    absence_short_circuits: IntCounter,
    ttl_purge_runs: IntCounterVec,
    ttl_purged_concepts: IntCounter,
    persistence_reconciliations: IntCounter,
    concepts_count: IntGauge,
    associations_count: IntGauge,
    cache_hit_ratio: IntGauge,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

/// Initialise the global metrics, registering them with the global
/// registry. Returns the static handle. Idempotent.
fn ensure_metrics() -> &'static Metrics {
    METRICS.get_or_init(|| {
        let r = registry();

        let probe_total = IntCounterVec::new(
            Opts::new("csm_probe_total", "Total number of probe calls."),
            &["result"],
        )
        .expect("counter construction");
        r.register(Box::new(probe_total.clone()))
            .expect("register probe_total");

        let probe_latency_ms = Histogram::with_opts(
            HistogramOpts::new("csm_probe_latency_ms", "Probe latency in milliseconds.")
                .buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0]),
        )
        .expect("histogram construction");
        r.register(Box::new(probe_latency_ms.clone()))
            .expect("register probe_latency_ms");

        let inject_total = IntCounterVec::new(
            Opts::new("csm_inject_total", "Total number of inject calls."),
            &["with_metadata"],
        )
        .expect("counter construction");
        r.register(Box::new(inject_total.clone()))
            .expect("register inject_total");

        let persist_latency_ms = HistogramVec::new(
            HistogramOpts::new(
                "csm_persist_latency_ms",
                "Persistence operation latency in milliseconds.",
            )
            .buckets(vec![0.5, 1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]),
            &["op"],
        )
        .expect("histogram construction");
        r.register(Box::new(persist_latency_ms.clone()))
            .expect("register persist_latency_ms");

        let import_failures = IntCounterVec::new(
            Opts::new("csm_import_failures_total", "Failed import operations."),
            &["format"],
        )
        .expect("counter construction");
        r.register(Box::new(import_failures.clone()))
            .expect("register import_failures");

        let stale_snapshot_rebuilds = IntCounterVec::new(
            Opts::new(
                "csm_stale_snapshot_rebuilds_total",
                "ANN snapshots rejected or rebuilt because cached state was stale or invalid.",
            ),
            &["reason"],
        )
        .expect("counter construction");
        r.register(Box::new(stale_snapshot_rebuilds.clone()))
            .expect("register stale_snapshot_rebuilds");

        let absence_short_circuits = IntCounter::new(
            "csm_absence_short_circuits_total",
            "Retrieval requests short-circuited by a scoped negative cache entry.",
        )
        .expect("counter construction");
        r.register(Box::new(absence_short_circuits.clone()))
            .expect("register absence_short_circuits");

        let ttl_purge_runs = IntCounterVec::new(
            Opts::new("csm_ttl_purge_total", "Completed TTL purge operations."),
            &["result"],
        )
        .expect("counter construction");
        r.register(Box::new(ttl_purge_runs.clone()))
            .expect("register ttl_purge_runs");

        let ttl_purged_concepts = IntCounter::new(
            "csm_ttl_purged_concepts_total",
            "Concepts removed by successful TTL purge operations.",
        )
        .expect("counter construction");
        r.register(Box::new(ttl_purged_concepts.clone()))
            .expect("register ttl_purged_concepts");

        let persistence_reconciliations = IntCounter::new(
            "csm_persistence_reconciliations_total",
            "Namespace reloads from authoritative persistence rows.",
        )
        .expect("counter construction");
        r.register(Box::new(persistence_reconciliations.clone()))
            .expect("register persistence_reconciliations");

        let concepts_count = IntGauge::new(
            "csm_concepts_count",
            "Number of active concepts in the in-memory store.",
        )
        .expect("gauge construction");
        r.register(Box::new(concepts_count.clone()))
            .expect("register concepts_count");

        let associations_count = IntGauge::new(
            "csm_associations_count",
            "Number of active associations in the in-memory store.",
        )
        .expect("gauge construction");
        r.register(Box::new(associations_count.clone()))
            .expect("register associations_count");

        let cache_hit_ratio = IntGauge::new(
            "csm_cache_hit_ratio",
            "Similarity cache hit ratio expressed as basis points (0-10000).",
        )
        .expect("gauge construction");
        r.register(Box::new(cache_hit_ratio.clone()))
            .expect("register cache_hit_ratio");

        Metrics {
            probe_total,
            probe_latency_ms,
            inject_total,
            persist_latency_ms,
            concepts_count,
            associations_count,
            cache_hit_ratio,
            import_failures,
            stale_snapshot_rebuilds,
            absence_short_circuits,
            ttl_purge_runs,
            ttl_purged_concepts,
            persistence_reconciliations,
        }
    })
}

/// Record one probe outcome (`result` = `ok` | `error`).
pub fn record_probe(result: &str, latency_ms: f64) {
    let m = ensure_metrics();
    m.probe_total.with_label_values(&[result]).inc();
    m.probe_latency_ms.observe(latency_ms);
}

/// Record one inject outcome.
pub fn record_inject(with_metadata: bool) {
    let m = ensure_metrics();
    let label = if with_metadata { "true" } else { "false" };
    m.inject_total.with_label_values(&[label]).inc();
}

/// Record one persistence operation outcome.
pub fn record_persist(op: &str, latency_ms: f64) {
    let m = ensure_metrics();
    m.persist_latency_ms
        .with_label_values(&[op])
        .observe(latency_ms);
}

/// Record a rejected import using a bounded format label.
pub(crate) fn record_import_failure(format: &str) {
    ensure_metrics()
        .import_failures
        .with_label_values(&[format])
        .inc();
}

/// Record a snapshot rebuild caused by stale or invalid derived state.
pub(crate) fn record_stale_snapshot_rebuild(reason: &str) {
    ensure_metrics()
        .stale_snapshot_rebuilds
        .with_label_values(&[reason])
        .inc();
}

/// Record a request skipped by a current, scoped negative-cache entry.
pub(crate) fn record_absence_short_circuit() {
    ensure_metrics().absence_short_circuits.inc();
}

/// Record one completed TTL purge and its removed concept count.
pub(crate) fn record_ttl_purge(result: &str, concepts: usize) {
    let metrics = ensure_metrics();
    metrics.ttl_purge_runs.with_label_values(&[result]).inc();
    if result == "success"
        && let Ok(count) = u64::try_from(concepts)
    {
        metrics.ttl_purged_concepts.inc_by(count);
    }
}

/// Record a reload from authoritative persistence rows.
pub(crate) fn record_persistence_reconciliation() {
    ensure_metrics().persistence_reconciliations.inc();
}

/// Update the size gauges (best-effort; failure is ignored).
pub fn set_concepts_count(n: i64) {
    ensure_metrics().concepts_count.set(n);
}

/// Update the associations gauge.
pub fn set_associations_count(n: i64) {
    ensure_metrics().associations_count.set(n);
}

/// Update the cache hit ratio gauge (`0..=10_000` basis points).
pub fn set_cache_hit_ratio(bps: i64) {
    ensure_metrics().cache_hit_ratio.set(bps);
}

/// Encode the registry using the text exposition format.
pub fn render() -> Result<String> {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    encoder
        .encode(&registry().gather(), &mut buf)
        .map_err(|e| MemoryError::Observability(format!("encode metrics: {e}")))?;
    String::from_utf8(buf).map_err(|e| MemoryError::Observability(format!("utf8: {e}")))
}

/// Handle to a running metrics HTTP server.
pub struct PromServerHandle {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
}

impl PromServerHandle {
    /// Local address the server is bound to. Useful when the caller passed
    /// port `0` for ephemeral binding.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Request graceful shutdown.
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

/// Start the metrics server bound to `bind`. Blocks until the server task
/// has accepted its socket so callers can rely on `local_addr`.
pub fn start_server(bind: SocketAddr) -> Result<PromServerHandle> {
    use http_body_util::Full;
    use hyper::body::Bytes;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;

    ensure_metrics();

    let listener = std::net::TcpListener::bind(bind)
        .map_err(|e| MemoryError::Observability(format!("bind {bind}: {e}")))?;
    let addr = listener
        .local_addr()
        .map_err(|e| MemoryError::Observability(format!("local_addr: {e}")))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| MemoryError::Observability(format!("set_nonblocking: {e}")))?;
    let tcp = tokio::net::TcpListener::from_std(listener)
        .map_err(|e| MemoryError::Observability(format!("tokio listener: {e}")))?;

    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut shutdown = rx;
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                accepted = tcp.accept() => {
                    let Ok((stream, _peer)) = accepted else { continue };
                    let io = TokioIo::new(stream);
                    let svc = service_fn(|_req: hyper::Request<hyper::body::Incoming>| async {
                        let body = match render() {
                            Ok(text) => text,
                            Err(e) => {
                                let msg = format!("# error rendering metrics: {e}\n");
                                return Ok::<_, std::convert::Infallible>(
                                    hyper::Response::builder()
                                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                                        .header(hyper::header::CONTENT_TYPE, "text/plain; version=0.0.4")
                                        .body(Full::new(Bytes::from(msg)))
                                        .expect("response builder"),
                                );
                            }
                        };
                        Ok::<_, std::convert::Infallible>(
                            hyper::Response::builder()
                                .status(hyper::StatusCode::OK)
                                .header(hyper::header::CONTENT_TYPE, "text/plain; version=0.0.4")
                                .body(Full::new(Bytes::from(body)))
                                .expect("response builder"),
                        )
                    });
                    let _ = http1::Builder::new().serve_connection(io, svc).await;
                }
            }
        }
    });

    Ok(PromServerHandle {
        addr,
        shutdown: Some(tx),
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_counter_at_least(metrics: &str, name: &str, minimum: f64) {
        let sample = metrics
            .lines()
            .find(|line| line.starts_with(name))
            .expect("counter sample should be rendered");
        let value = sample
            .split_whitespace()
            .nth(1)
            .expect("counter sample should include a value")
            .parse::<f64>()
            .expect("counter sample should contain a numeric value");
        assert!(
            value >= minimum,
            "{name} expected at least {minimum}, got {value}"
        );
    }

    #[test]
    fn operational_counters_are_exposed_with_bounded_labels() {
        record_import_failure("json");
        record_stale_snapshot_rebuild("revision_mismatch");
        record_absence_short_circuit();
        record_ttl_purge("success", 2);
        record_persistence_reconciliation();

        let metrics = render().expect("metrics should render");
        assert_counter_at_least(&metrics, "csm_import_failures_total{format=\"json\"}", 1.0);
        assert_counter_at_least(
            &metrics,
            "csm_stale_snapshot_rebuilds_total{reason=\"revision_mismatch\"}",
            1.0,
        );
        assert_counter_at_least(&metrics, "csm_absence_short_circuits_total", 1.0);
        assert_counter_at_least(&metrics, "csm_ttl_purge_total{result=\"success\"}", 1.0);
        assert_counter_at_least(&metrics, "csm_ttl_purged_concepts_total", 2.0);
        assert_counter_at_least(&metrics, "csm_persistence_reconciliations_total", 1.0);
    }
}
