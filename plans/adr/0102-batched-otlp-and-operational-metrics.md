# ADR-0102: Bounded OTLP Lifecycle and Operational Counters

## Status

Accepted

## Context and Problem Statement

The OTLP gRPC exporter previously used a simple span processor and discarded shutdown errors. That made request latency depend on synchronous export and left no explicit bound for buffered spans or provider shutdown. Operational metrics also lacked counters for import errors, stale ANN snapshots, negative-cache short-circuits, TTL cleanup, and authoritative-row reconciliation.

## Decision Drivers

- Keep application work independent of remote exporter latency.
- Bound span queue, batch size, individual export/force-flush duration, and shutdown.
- Make provider flush/shutdown observable and one-time.
- Keep operational counters low-cardinality and tied to real state transitions.

## Considered Options

- Retain the simple processor and best-effort Drop behavior.
- Use a batch processor with library defaults and ignore shutdown errors.
- Use a configurable bounded batch processor, explicit guard lifecycle, and fixed-label operational counters.

## Decision Outcome

Chosen option: **configurable batch export with bounded lifecycle and low-cardinality counters**, because it states operational bounds, surfaces lifecycle failures, and avoids user-controlled metric cardinality.

`OtlpBatchConfig` defaults to a 2,048-span queue, 512-span export batch, five-second scheduled delay, five-second export-request timeout, and five-second provider shutdown timeout. The export timeout is applied to the OTLP gRPC channel, because the SDK's `BatchConfigBuilder::with_max_export_timeout` exists only behind its experimental async-runtime processor; one request therefore fails on its own deadline, while a force flush that drains several batches can exceed that bound in total. Queue saturation follows the OpenTelemetry processor's drop-new-span behavior. `Guard::force_flush` reports exporter errors; `Guard::shutdown` takes the provider exactly once and returns shutdown failures; `Drop` logs errors instead of suppressing them. Initialization validates configuration before locking the one-time init state and resets that state on startup failure.

Prometheus exposes counters for JSON/binary import failures, stale ANN snapshot rebuild reasons, absence short-circuits, TTL purge outcome and purged concept count, and persistence reconciliation. Labels are fixed low-cardinality values supplied by internal call sites, not user-controlled data.

### Positive Consequences

- Span export is asynchronous and bounded in queue capacity; each remote request carries its own deadline.
- Applications can flush or shut down explicitly and observe failures.
- Operational counters expose important correctness/lifecycle events without high-cardinality identifiers.
- Invalid config or failed startup does not permanently poison observability initialization.

### Negative Consequences

- Spans may be dropped when the configured bounded queue is full.
- Callers must retain the guard and explicitly handle shutdown errors when deterministic delivery matters.
- Counter label values are an internal contract and must remain bounded.

## Pros and Cons of the Options

### Simple processor and best-effort Drop

- Good, because it has minimal configuration and straightforward tests.
- Bad, because remote export can block application work and shutdown failures are invisible.

### Batch defaults with ignored errors

- Good, because it improves throughput with little implementation code.
- Bad, because defaults do not state this service's operational bounds and errors remain hidden.

### Configured batch processor and explicit guard

- Good, because queue, export, flush, and shutdown limits are explicit and errors are actionable.
- Bad, because the API gains configuration and callers must choose lifecycle ownership.

## Follow-up and Verification

- Test invalid queue/batch/timeout bounds and idempotent empty-guard flush/shutdown.
- Verify emitted operational counters through the Prometheus exposition format.
- Keep OTLP integration tests feature-gated and exercise the real provider lifecycle in a collector-backed test before making network export a default.
