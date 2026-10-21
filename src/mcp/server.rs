//! MCP Server implementation using rmcp (ADR-0067)
//!
//! Provides stdio and Streamable HTTP transports for MCP clients.

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use tracing::info;

use crate::mcp::handler::McpHandler;

/// Transport type for MCP server.
#[derive(Debug, Clone, Copy, Default)]
pub enum Transport {
    /// Standard input/output (default for desktop apps)
    #[default]
    Stdio,
    /// Streamable HTTP transport.
    StreamableHttp {
        /// Bind address.
        bind: std::net::SocketAddr,
    },
}

/// Transport type for CLI parsing.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum TransportType {
    /// Standard input/output (default for desktop apps)
    #[default]
    Stdio,
    /// Streamable HTTP transport.
    StreamableHttp,
}

/// Configuration for MCP server.
#[derive(Clone)]
pub struct McpConfig {
    /// Transport type.
    pub transport: Transport,
    /// Bind address for Streamable HTTP.
    pub bind: Option<String>,
    /// Database path.
    pub database: Option<std::path::PathBuf>,
    /// Optional HTTP bearer token. Required for non-loopback binds.
    pub auth_token: Option<String>,
    /// Explicit allowed HTTP Host values; unset uses loopback/bind defaults.
    pub allowed_hosts: Option<Vec<String>>,
    /// Allowed browser origins. Present origins are rejected when empty.
    pub allowed_origins: Vec<String>,
    /// Seconds between background TTL cleanup passes for the framework this
    /// server builds; `0` (the default) leaves the reaper off, so a long-running
    /// `csm mcp serve` never starts deleting expired concepts silently. Set from
    /// `--ttl-cleanup-interval` (ADR-0099).
    pub ttl_cleanup_interval: u64,
}

impl std::fmt::Debug for McpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConfig")
            .field("transport", &self.transport)
            .field("bind", &self.bind)
            .field("database", &self.database)
            .field(
                "auth_token",
                &self.auth_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("ttl_cleanup_interval", &self.ttl_cleanup_interval)
            .field("allowed_hosts", &self.allowed_hosts)
            .field("allowed_origins", &self.allowed_origins)
            .finish()
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            bind: None,
            database: None,
            auth_token: None,
            allowed_hosts: None,
            allowed_origins: Vec::new(),
            ttl_cleanup_interval: 0,
        }
    }
}

/// Start the MCP server.
///
/// The exit signals are the operator's Ctrl+C (SIGINT) and a service manager's
/// SIGTERM — see [`serve_with_shutdown`] for a variant driven by the caller's own
/// lifecycle.
///
/// # Errors
///
/// Returns error if server fails to start or transport initialization fails.
pub async fn serve(config: McpConfig) -> Result<()> {
    // ADR-0099: Streamable HTTP needs a real shutdown signal because its
    // transport future otherwise never resolves (see
    // `run_streamable_http_server_with_shutdown`). Both the foreground
    // operator's SIGINT and a service manager's SIGTERM must reach it. Stdio
    // ends on stdin EOF and needs no additional signal.
    serve_with_shutdown(config, crate::shutdown::operator_shutdown()).await
}

/// Start the MCP server with an explicit shutdown signal for Streamable HTTP.
///
/// [`serve`] is exactly `serve_with_shutdown(config, operator_shutdown())`, so the
/// production path and this entry share one mechanism — the signal is not a
/// test-only seam. It exists for embedders that run the server inside a larger
/// lifecycle (a supervisor, a co-located HTTP app, a service manager) and cannot
/// rely on being the foreground process that receives SIGINT.
///
/// The signal drives Streamable HTTP only. Stdio ends on stdin EOF and ignores
/// it, deliberately: that transport already has a working exit path, and racing
/// two of them would change behaviour nobody asked for.
///
/// # Errors
///
/// Returns error if server fails to start or transport initialization fails.
pub async fn serve_with_shutdown<F>(config: McpConfig, shutdown_signal: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    info!("Starting MCP server with {:?} transport", config.transport);

    if config
        .auth_token
        .as_ref()
        .is_some_and(std::string::String::is_empty)
    {
        anyhow::bail!("MCP bearer token must not be empty");
    }
    if let Transport::StreamableHttp { bind } = config.transport
        && !bind.ip().is_loopback()
        && config.auth_token.is_none()
    {
        anyhow::bail!(
            "non-loopback MCP binds require CSM_MCP_BEARER_TOKEN or an explicit auth_token"
        );
    }

    if config.allowed_hosts.as_ref().is_some_and(Vec::is_empty) {
        anyhow::bail!("an explicit MCP allowed-host list must not be empty");
    }

    let auth_token = config.auth_token;
    let allowed_hosts = config.allowed_hosts;
    let allowed_origins = config.allowed_origins;
    let handler = Arc::new(
        McpHandler::new(config.database).with_ttl_cleanup_interval(config.ttl_cleanup_interval),
    );
    serve_with_handler(
        handler,
        config.transport,
        auth_token,
        allowed_hosts,
        allowed_origins,
        shutdown_signal,
    )
    .await
}
/// Body of [`serve_with_shutdown`] with the handler supplied by the caller.
///
/// Kept crate-internal because the ADR-0099 guarantee is only observable through
/// the handler: a test that injects the handler it also inspects can assert the
/// shared TTL cleanup task is really gone after the transport resolves, instead
/// of asserting the config echo. Production reaches this through `serve`.
async fn serve_with_handler<F>(
    handler: Arc<McpHandler>,
    transport: Transport,
    auth_token: Option<String>,
    allowed_hosts: Option<Vec<String>>,
    allowed_origins: Vec<String>,
    shutdown_signal: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    match transport {
        Transport::Stdio => {
            let (stdin, stdout) = rmcp::transport::io::stdio();
            // `Arc<McpHandler>` implements `ServerHandler`, so the same
            // framework instance can be shut down after the transport ends.
            let server = rmcp::serve_server(handler.clone(), (stdin, stdout)).await?;
            server
                .waiting()
                .await
                .map_err(|e| anyhow::anyhow!("Server join error: {e}"))?;
        }
        Transport::StreamableHttp { bind } => {
            let listener = tokio::net::TcpListener::bind(bind).await?;
            info!(
                "MCP Streamable HTTP server listening on http://{}",
                listener.local_addr()?
            );
            run_streamable_http_server_with_shutdown(
                handler.clone(),
                listener,
                auth_token,
                allowed_hosts,
                allowed_origins,
                shutdown_signal,
            )
            .await?;
        }
    }

    // ADR-0099: the handler owns the framework — and with it the shared TTL
    // cleanup task — in a `OnceCell`, so shutting it down here is what stops
    // the task deterministically on exit instead of at its next cooperative
    // check. A server that served no request never initialized the framework,
    // in which case this is a no-op.
    handler
        .shutdown()
        .await
        .map_err(|e| anyhow::anyhow!("TTL cleanup shutdown failed: {e}"))?;

    Ok(())
}

/// Run the Streamable HTTP transport until the listener fails or
/// `shutdown_signal` resolves.
///
/// The signal is load-bearing, not an optimisation: without it this function can
/// never return. In axum 0.7.9 `Serve::into_future` is an unbounded accept loop
/// (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/axum-0.7.9/src/serve.rs:205-240`)
/// and `tcp_accept` reports failures as `Option` — connection errors are skipped,
/// anything else is logged and retried after a sleep — so it never yields an
/// `Err` the caller could observe (`:474-496`). `with_graceful_shutdown`
/// (`:139-141`) is the only exit: it stops accepting, drops the listener, asks
/// in-flight connections to finish, and resolves `Ok(())` once they have
/// (`:348-463`). That return is what makes the ADR-0099 shutdown hop in
/// [`serve_with_handler`] reachable on this transport.
async fn run_streamable_http_server_with_shutdown<F>(
    handler: Arc<McpHandler>,
    listener: tokio::net::TcpListener,
    auth_token: Option<String>,
    allowed_hosts: Option<Vec<String>>,
    allowed_origins: Vec<String>,
    shutdown_signal: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };

    let bind = listener.local_addr()?;
    let allowed_hosts = allowed_hosts.unwrap_or_else(|| {
        vec![
            "localhost".to_string(),
            "127.0.0.1".to_string(),
            bind.ip().to_string(),
            format!("{}:{}", bind.ip(), bind.port()),
        ]
    });
    let config = StreamableHttpServerConfig::default()
        .with_allowed_hosts(allowed_hosts)
        .with_allowed_origins(allowed_origins)
        .enforce_origin_validation()
        .with_json_response(true);

    // rmcp routes 2026-07-28 stateless requests directly and uses this manager
    // only for negotiated legacy protocol versions.
    let session_manager = Arc::new(LocalSessionManager::default());
    let service_factory = move || Ok(handler.clone());
    let service = StreamableHttpService::new(service_factory, session_manager, config);

    let app = axum::Router::new().fallback_service(service);
    let app = if let Some(token) = auth_token {
        app.layer(axum::middleware::from_fn_with_state(
            Arc::<str>::from(token),
            require_bearer,
        ))
    } else {
        app
    }
    .layer(axum::middleware::from_fn(trace_context));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| anyhow::anyhow!("axum server error: {e}"))?;

    Ok(())
}

async fn require_bearer(
    axum::extract::State(expected): axum::extract::State<Arc<str>>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    use subtle::ConstantTimeEq as _;

    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let authorized =
        token.is_some_and(|token| bool::from(expected.as_bytes().ct_eq(token.as_bytes())));
    if !authorized {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            [(axum::http::header::WWW_AUTHENTICATE, "Bearer")],
            "Unauthorized",
        )
            .into_response();
    }

    next.run(request).await
}

async fn trace_context(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use tracing::Instrument as _;

    let span = tracing::info_span!("mcp_http_request", traceparent = tracing::field::Empty);
    if let Some(traceparent) = request
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
    {
        span.record("traceparent", traceparent);
    }

    #[cfg(all(feature = "otlp", not(target_arch = "wasm32")))]
    {
        use tracing_opentelemetry::OpenTelemetrySpanExt as _;
        // `LayerNotFound`/`SpanDisabled` are the normal outcome when no OTLP
        // layer is installed, so the parent is best-effort and never fails the
        // request.
        if let Err(error) = span.set_parent(extract_trace_parent(request.headers())) {
            tracing::debug!(%error, "traceparent was not applied to the request span");
        }
    }

    next.run(request).instrument(span).await
}

#[cfg(all(feature = "otlp", not(target_arch = "wasm32")))]
fn extract_trace_parent(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
    struct HeaderExtractor<'a>(&'a axum::http::HeaderMap);
    impl opentelemetry::propagation::Extractor for HeaderExtractor<'_> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|value| value.to_str().ok())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(axum::http::HeaderName::as_str).collect()
        }
    }

    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
