# ADR-0078: CloudEvents Event Emitter

## Status

Proposed (2026-05-04)

## Context and Problem Statement

The `ChaoticSemanticFramework` currently emits internal `MemoryEvent` notifications via a `tokio::sync::broadcast` channel. While this is sufficient for in-process subscribers, it does not support distributed AI agent architectures where memory changes must trigger external workflows.

CloudEvents is a CNCF specification for describing event data in a common way. Standardizing our memory events as CloudEvents enables:
- **Interoperability** — Triggering serverless functions, webhooks, or event buses (NATS, Kafka).
- **Tooling** — Using standard CloudEvents SDKs for downstream consumers.
- **Auditability** — Structured event logs with standard metadata (id, source, type, time).

## Decision Drivers

- Standardize event schema across native and WASM boundaries (where applicable).
- Minimize overhead: CloudEvent conversion should be cheap.
- Pluggable sinks: Support multiple output targets (Stdout, HTTP, etc.).
- Maintain LOC limits (≤ 400 lines for the new implementation file).

## Considered Options

1. **Native CloudEvents conversion** in `src/framework_events.rs`.
2. **Pluggable `EventEmitter` trait** with `MemoryEvent` → `CloudEvent` mapping.
3. **External "sidecar"** that subscribes to the broadcast channel and re-emits.

## Decision Outcome

Chosen: **Option 2** — Pluggable `EventEmitter` trait.

This approach provides a clean separation between internal broadcast events and external standardized events. It allows the framework to remain unopinionated about the event sink while providing a first-class way to integrate with the CloudEvents ecosystem.

## Implementation

### New Module

`src/framework_events_ce.rs`

### Trait Definition

```rust
#[async_trait]
pub trait EventEmitter: Send + Sync {
    fn name(&self) -> &str;
    async fn emit(&self, event: cloudevents::Event) -> Result<(), MemoryError>;
}
```

### Event Mapping

`MemoryEvent` will be mapped to `cloudevents::Event` with:
- `source`: `chaotic-semantic-memory://<namespace>`
- `type`: `io.d-o-hub.csm.memory.<variant>` (e.g., `concept.injected`)
- `subject`: The concept ID.
- `data`: JSON representation of the event payload.

### Built-in Emitters

1. **LogEmitter** — Emits formatted CloudEvents to the `tracing` log.
2. **HttpEmitter** — Posts CloudEvents to a configured webhook URL (native only, requires `reqwest`).

### Integration

`ChaoticSemanticFramework` will optionally hold a list of emitters:

```rust
pub struct ChaoticSemanticFramework {
    // ...
    emitters: Vec<Box<dyn EventEmitter>>,
}
```

When an event is emitted internally, the framework will iterate through configured emitters and trigger their `emit` methods.

## Pros and Cons

### Pros
- Compliance with industry standard event formats.
- Highly extensible sink architecture.
- Improved observability and auditability for distributed agents.

### Cons
- Added dependency on `cloudevents-sdk`.
- Minor latency overhead during mutation operations (async emission).

## Acceptance Criteria

- [ ] `EventEmitter` trait defined in `src/framework_events_ce.rs`.
- [ ] `MemoryEvent` to `cloudevents::Event` mapping logic implemented.
- [ ] `LogEmitter` provided as a default implementation.
- [ ] `HttpEmitter` provided behind an opt-in feature.
- [ ] Integration tests verify that mutating operations trigger the emitter.
- [ ] Documentation updated to show how to configure custom emitters.
- [ ] File size in `src/` remains under 400 lines.
