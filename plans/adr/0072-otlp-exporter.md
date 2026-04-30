# ADR-0072: OpenTelemetry OTLP Exporter

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`tracing` instrumentation is in place across reservoir, persistence, CLI, and singularity (70% coverage per GOAP_STATE). However, the only sink today is `tracing_subscriber::FmtSubscriber` writing to stderr.

Production deployments need:
- **OTLP gRPC** export to Jaeger / Tempo / Honeycomb / Datadog
- **Prometheus** `/metrics` scrape endpoint (counters, histograms)
- **Structured log shipping** (JSON over stdout for log shippers)

Currently each operator must wire their own subscriber stack. Comparable systems (`qdrant`, `weaviate`) ship first-class OTLP integration.

## Decision Drivers

- Opt-in (default unchanged)
- WASM unaffected
- Spans for hot path operations: probe, inject, persist, traverse
- Counters: probe_count, inject_count, error_count
- Histograms: probe_latency_ms, persist_latency_ms

## Considered Options

1. **First-party `observability` feature** with OTLP + Prometheus exporters
2. Document the wiring pattern only
3. Side-car library

## Decision Outcome

Chosen: **Option 1** — first-party feature. Avoids "the observability story is a tutorial" trap.

## Implementation

### Cargo features

```toml
[features]
otlp = ["dep:opentelemetry", "dep:opentelemetry-otlp", "dep:opentelemetry_sdk", "dep:tracing-opentelemetry"]
prometheus = ["dep:prometheus", "dep:hyper"]
```

### New module

`src/observability/` (3 files, each ≤ 300 LOC):

| File | Responsibility |
|---|---|
| `mod.rs` | `init()` entry point + config |
| `otlp.rs` | OTLP gRPC/HTTP exporter wiring |
| `prom.rs` | Prometheus metrics + HTTP scrape endpoint |

### Public API

```rust
pub struct ObservabilityConfig {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,    // e.g., "http://localhost:4317"
    pub prometheus_bind: Option<SocketAddr>,
    pub log_format: LogFormat,            // Pretty | Json | Compact
}

pub fn init(config: ObservabilityConfig) -> Result<Guard>;
```

### Metrics surfaced

| Metric | Type | Labels |
|---|---|---|
| `csm_probe_total` | counter | result=ok\|error |
| `csm_probe_latency_ms` | histogram | top_k_bucket |
| `csm_inject_total` | counter | with_metadata=true\|false |
| `csm_persist_latency_ms` | histogram | op=load\|save\|migrate |
| `csm_concepts_count` | gauge | — |
| `csm_associations_count` | gauge | — |
| `csm_cache_hit_ratio` | gauge | — |

### CLI integration

```
csm --otlp-endpoint http://localhost:4317 --prometheus-bind 127.0.0.1:9090 stats
```

Or via env: `CSM_OTLP_ENDPOINT`, `CSM_PROMETHEUS_BIND`.

### Example config

`examples/observability_otlp.rs` — boots framework with OTLP, runs probes, prints OTLP collector commands.

## Pros and Cons

### Pros
- Production-ready out of the box
- Opt-in (no perf cost when disabled)
- Standard wire format (OTLP) so any APM works

### Cons
- Adds 3 optional deps (~300 KB compiled)
- Init order matters (must run before first framework op)
- Prometheus pull-model only; push gateway not included

## Acceptance Criteria

- [ ] `cargo build --features otlp` succeeds
- [ ] `cargo build --features prometheus` succeeds
- [ ] Smoke test against local Jaeger: spans visible
- [ ] Smoke test against local Prometheus: 7 metrics scraped
- [ ] `examples/observability_otlp.rs` runs
- [ ] Each `src/observability/*.rs` file ≤ 300 LOC
