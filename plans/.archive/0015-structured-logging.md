# [ADR-0015] Structured Logging with Tracing

## Status
Accepted

## Context and Problem Statement
Current error handling uses `thiserror` but lacks operational logging. Production deployments need visibility into:
- Operation latencies
- Error rates
- System behavior under load

## Decision Drivers
- Need observability for production deployments
- Must integrate with existing error types
- Should support both human-readable and structured (JSON) output
- Must be optional/zero-cost when disabled

## Considered Options
1. **log crate** - Standard logging, simple but limited
2. **tracing** - Structured logging with spans, async-aware
3. **slog** - Structured logging, older ecosystem
4. **No logging** - Leave to application layer

## Decision Outcome
Chosen option: **tracing** for structured, async-aware logging

### Implementation Strategy
- Add `tracing` as optional dependency
- `#[instrument]` on async framework methods
- Spans for persistence operations
- Configurable via `tracing-subscriber`
- JSON output via `tracing-subscriber::fmt::json()`

### Positive Consequences
- Rich context in logs (spans, fields)
- Async-aware (tracks across await points)
- OpenTelemetry compatible for future integration
- Zero-cost when disabled

### Negative Consequences
- Additional dependency
- Learning curve for contributors
- Potential log volume in high-throughput scenarios

## Links
- [tracing documentation](https://docs.rs/tracing/)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
