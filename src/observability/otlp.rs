//! JSON log subscriber (ADR-0072, opt-in via `otlp-json` feature).
//!
//! Installs a `tracing_subscriber` JSON formatter that writes one
//! newline-delimited JSON object per event to stdout. This shape is what
//! the [`opentelemetry-log`](https://docs.rs/opentelemetry-stdout/) exporter
//! and most log shippers (Fluent Bit, Vector, Promtail, Loki) expect on
//! the wire — the lightweight alternative to a full `opentelemetry-otlp`
//! gRPC exporter called out in ADR-0072 §"Implementation".
//!
//! # Design
//!
//! - Reuses `tracing_subscriber::fmt::format::Json` (already in
//!   `tracing-subscriber`'s `json` feature). No new dependencies.
//! - Best-effort: a `set_global_default` failure is intentionally ignored
//!   because the most common cause is a competing subscriber (e.g. a test
//!   harness). Returning the error to the caller would break otherwise
//!   working applications.

use tracing_subscriber::fmt::writer::MakeWriter;

/// Writer that sends events to stdout. Defined as its own type so the
/// JSON subscriber does not depend on the default `tracing_subscriber`
/// `Stdout` target (which can be re-initialised across tests).
#[derive(Debug, Default, Clone, Copy)]
pub struct StdoutWriter;

impl<'a> MakeWriter<'a> for StdoutWriter {
    type Writer = StdoutGuard;
    fn make_writer(&'a self) -> Self::Writer {
        StdoutGuard
    }
}

/// `Write` adapter that writes to locked stdout.
pub struct StdoutGuard;

impl std::io::Write for StdoutGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut out = std::io::stdout().lock();
        out.write_all(buf)?;
        out.flush()?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

/// Install a global JSON tracing subscriber.
///
/// Idempotent at the level the caller cares about: if a subscriber is
/// already installed (`set_global_default` fails), the function returns
/// without touching anything. This is the same pattern used by
/// `tracing_subscriber::fmt::init()` and keeps the public API panic-free.
pub fn install_json_subscriber() {
    let _ = tracing_subscriber::fmt()
        .json()
        .with_writer(StdoutWriter)
        .with_current_span(true)
        .with_span_list(false)
        .with_target(true)
        .with_level(true)
        .flatten_event(true)
        .try_init();
}
