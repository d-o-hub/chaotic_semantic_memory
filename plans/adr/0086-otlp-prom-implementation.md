# ADR-0086: OTLP / Prometheus Implementation Notes (ADR-0072)

## Status

Implemented (2026-06-06). Builds on ADR-0072.

## Context

ADR-0072 proposed a first-party `observability` feature behind
`prometheus` and `otlp` (later renamed `otlp-json`) features, with seven
metrics and a JSON log subscriber. The original ADR listed `opentelemetry`,
`opentelemetry-otlp`, and `tracing-opentelemetry` as the implementation
stack for the OTLP gRPC exporter.

During implementation (branch `feat/observability-otlp-prom`), the
`otlp` feature was renamed to `otlp-json` and the gRPC exporter was
deliberately scoped out. The custom JSON subscriber attempted in the
first pass also turned out to be a re-implementation of what
`tracing_subscriber::fmt::format::Json` already provides.

## Decision

1. **Two features, two exports:**
   - `prometheus` — adds the `prometheus`, `hyper`, `hyper-util`, and
     `http-body-util` deps, builds the `csm_*` registry, and serves
     `GET /metrics` on the configured `SocketAddr`.
   - `otlp-json` — adds the `tracing-subscriber` `json` feature and
     installs a global JSON formatter on stdout. No additional
     dependencies; the JSON shape is the standard `tracing-subscriber`
     output (one object per line, parseable by Fluent Bit / Vector /
     Promtail / `opentelemetry-stdout`).

2. **gRPC OTLP deferred.** The `opentelemetry-otlp` stack would add
   ~300 KB compiled and a transitive `tonic` / `prost` / `protobuf`
   pull. The `otlp-json` path covers the same operational use case
   (log shipping to a collector that forwards to OTLP) without the
   compile cost. A future ADR can layer the gRPC exporter on top of
   the same `ObservabilityConfig` struct.

3. **No framework-side auto-wiring (yet).** The `prom::record_*` and
   `prom::set_*` helpers are public so callers can wire the framework
   hot path themselves (see `examples/observability_otlp.rs`). Auto-
   wiring (`#[instrument]` → `record_probe` inside `framework.rs` and
   `singularity.rs`) is a separate, narrower change. Tracked as a
   follow-up in `plans/GOAP_STATE.md` and
   `plans/GOAP_LIFECYCLE_VERIFICATION_FOLLOWUP.md`-style backlog.

## Implementation

### Files

| File | LOC (under 300 gate) | Responsibility |
|---|---|---|
| `src/observability/mod.rs` | 171 | `init()`, `Guard`, `LogFormat`, `ObservabilityConfig`, `render_metrics()` |
| `src/observability/otlp.rs` | 65 | JSON subscriber installer (gated on `otlp-json`) |
| `src/observability/prom.rs` | 250 | `prometheus` registry, server, metric helpers (gated on `prometheus`) |
| `tests/observability_integration.rs` | new | 6 tests, gated on the union of both features |
| `examples/observability_otlp.rs` | new | end-to-end smoke example |

### Public API

```rust
use chaotic_semantic_memory::observability::{
    self, LogFormat, ObservabilityConfig, init, prom, render_metrics,
};

let _guard = init(ObservabilityConfig {
    service_name: "csm-service".into(),
    otlp_endpoint: Some("http://localhost:4317".into()), // reserved, see below
    prometheus_bind: Some("127.0.0.1:9090".parse()?),
    log_format: LogFormat::Json,
})?;

// Caller-driven metric updates (auto-wiring is a follow-up):
prom::record_probe("ok", elapsed_ms);
prom::record_inject(false);
prom::record_persist("save", latency_ms);
prom::set_concepts_count(n);
```

### `otlp_endpoint` is reserved

The field is accepted and logged in the `tracing::info!` banner, but is
**not** used to open any network connection. It is intentionally
reserved for a future gRPC exporter so the configuration struct does
not need to break compatibility when that lands.

If a user wants to forward the JSON logs to an OTLP collector today,
the recommended path is a log shipper (Fluent Bit / Vector / Promtail)
consuming stdout and forwarding to the collector's OTLP receiver.

## Acceptance criteria (from ADR-0072)

- [x] `cargo build --features prometheus` succeeds
- [x] `cargo build --features otlp-json` succeeds
- [x] `cargo build --features prometheus,otlp-json` succeeds
- [x] `cargo build --no-default-features --features prometheus` succeeds
- [x] `cargo build --no-default-features --features otlp-json` succeeds
- [x] `examples/observability_otlp.rs` runs end-to-end (verified locally)
- [x] Each `src/observability/*.rs` file ≤ 300 LOC

## Smoke test (manual)

```bash
cargo build --example observability_otlp --features prometheus,otlp-json
./target/debug/examples/observability_otlp &
curl -s http://127.0.0.1:9090/metrics | grep csm_ | head
# csm_inject_total{with_metadata="false"} 16
# csm_probe_total{result="ok"} 1
# csm_concepts_count 16
# csm_probe_latency_ms_sum 1.77
```

The JSON subscriber writes one object per event to stdout — visible in
the example's `observability initialised` line.

## Follow-ups

1. **Auto-wiring** — call `prom::record_*` from the `#[instrument]`
   sites in `framework.rs` / `singularity.rs` so users do not have to
   do it themselves. Low risk, narrow surface.
2. **OTLP gRPC exporter** — when user demand materialises, layer
   `opentelemetry-otlp` behind a new `otlp` feature, share the
   `ObservabilityConfig::otlp_endpoint` field, and reuse the existing
   registry. The `otlp-json` path stays as a no-dep option.
3. **WASM** — the module compiles to `cfg(any(feature = "prometheus",
   feature = "otlp-json"))`. On `wasm32-unknown-unknown` the
   `prometheus` feature still compiles (the HTTP server task is
   feature-flagged at the source level too) but the runtime needs
   `tokio` with `net` and is unlikely to be useful in a browser
   context. Documented as "native-only" for the HTTP server; the JSON
   subscriber is harmless on WASM but stdout is not useful in the
   browser, so callers should not enable `otlp-json` for WASM builds.
