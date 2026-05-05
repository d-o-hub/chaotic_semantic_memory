//! Prometheus metrics exposition (ADR-0072).

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{
    Counter, CounterVec, Encoder, Gauge, HistogramVec, Opts, Registry, TextEncoder,
    register_counter_vec_with_registry, register_gauge_with_registry,
    register_histogram_vec_with_registry,
};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::LazyLock;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Global Prometheus registry.
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

/// Counter for probes: csm_probe_total{result="ok|error"}
pub static PROBE_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec_with_registry!(
        Opts::new("csm_probe_total", "Total number of similarity probes"),
        &["result"],
        *REGISTRY
    )
    .unwrap()
});

/// Histogram for probe latency: csm_probe_latency_ms
pub static PROBE_LATENCY: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec_with_registry!(
        "csm_probe_latency_ms",
        "Similarity probe latency in milliseconds",
        &["top_k"],
        vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0],
        *REGISTRY
    )
    .unwrap()
});

/// Counter for injections: csm_inject_total{with_metadata="true|false"}
pub static INJECT_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec_with_registry!(
        Opts::new("csm_inject_total", "Total number of concept injections"),
        &["with_metadata"],
        *REGISTRY
    )
    .unwrap()
});

/// Histogram for persistence latency: csm_persist_latency_ms{op="load|save|migrate"}
pub static PERSIST_LATENCY: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec_with_registry!(
        "csm_persist_latency_ms",
        "Persistence operation latency in milliseconds",
        &["op"],
        vec![10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0],
        *REGISTRY
    )
    .unwrap()
});

/// Counter for associations: csm_associations_total
pub static ASSOCIATIONS_TOTAL: LazyLock<Counter> = LazyLock::new(|| {
    register_counter_vec_with_registry!(
        Opts::new(
            "csm_associations_total",
            "Total number of associations created"
        ),
        &[],
        *REGISTRY
    )
    .unwrap()
    .with_label_values(&[])
});

/// Gauge for concept count: csm_concepts_count
pub static CONCEPTS_COUNT: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge_with_registry!(
        "csm_concepts_count",
        "Total number of concepts in memory",
        *REGISTRY
    )
    .unwrap()
});

/// Gauge for association count: csm_associations_count
pub static ASSOCIATIONS_COUNT: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge_with_registry!(
        "csm_associations_count",
        "Total number of associations in memory",
        *REGISTRY
    )
    .unwrap()
});

/// Gauge for cache hit ratio: csm_cache_hit_ratio
pub static CACHE_HIT_RATIO: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge_with_registry!(
        "csm_cache_hit_ratio",
        "Singularity query cache hit ratio (0.0-1.0)",
        *REGISTRY
    )
    .unwrap()
});

/// Initialize Prometheus exporter.
pub fn init_prometheus(bind_addr: SocketAddr) {
    tokio::spawn(async move {
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind Prometheus exporter to {}: {}", bind_addr, e);
                return;
            }
        };
        info!("Prometheus exporter listening on http://{}", bind_addr);

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };
            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(metrics_handler))
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }
    });
}

async fn metrics_handler(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Full::new(Bytes::from(buffer)))
        .unwrap())
}
