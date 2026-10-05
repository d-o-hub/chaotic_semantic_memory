//! MCP Server implementation using rmcp (ADR-0067)
//!
//! Provides stdio and SSE transports for Claude Desktop, Cursor, and other MCP clients.

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
    /// SSE transport
    Sse {
        /// Bind address
        bind: std::net::SocketAddr,
    },
}

/// Transport type for CLI parsing.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum TransportType {
    /// Standard input/output (default for desktop apps)
    #[default]
    Stdio,
    /// SSE transport
    Sse,
}

/// Configuration for MCP server.
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Transport type
    pub transport: Transport,
    /// Bind address for SSE transport
    pub bind: Option<String>,
    /// Database path
    pub database: Option<std::path::PathBuf>,
    /// Seconds between background TTL cleanup passes for the framework this
    /// server builds; `0` (the default) leaves the reaper off, so a long-running
    /// `csm mcp serve` never starts deleting expired concepts silently. Set from
    /// `--ttl-cleanup-interval` (ADR-0099).
    pub ttl_cleanup_interval: u64,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            bind: None,
            database: None,
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
    // ADR-0099: SSE needs a real shutdown signal because its transport future
    // otherwise never resolves (see `run_sse_server_with_shutdown`). Both the
    // foreground operator's SIGINT and a service manager's SIGTERM must reach it:
    // `systemctl stop` / `docker stop` send SIGTERM, and handling only SIGINT left
    // the bounded cleanup stop unreached under a service manager. Stdio does not
    // need it: rmcp's `Waiting::waiting` already resolves on stdin EOF.
    serve_with_shutdown(config, crate::shutdown::operator_shutdown()).await
}

/// Start the MCP server with an explicit shutdown signal for the SSE transport.
///
/// [`serve`] is exactly `serve_with_shutdown(config, operator_shutdown())`, so the
/// production path and this entry share one mechanism — the signal is not a
/// test-only seam. It exists for embedders that run the server inside a larger
/// lifecycle (a supervisor, a co-located HTTP app, a service manager) and cannot
/// rely on being the foreground process that receives SIGINT.
///
/// The signal drives the SSE transport only. Stdio ends on stdin EOF and ignores
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

    // The reaper interval is carried on the handler, not the transport, because
    // the framework is created lazily on the first tool call — this is the only
    // place that knows both the config value and the handler that builds it.
    let handler = Arc::new(
        McpHandler::new(config.database).with_ttl_cleanup_interval(config.ttl_cleanup_interval),
    );

    serve_with_handler(handler, config.transport, shutdown_signal).await
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
        Transport::Sse { bind } => {
            let listener = tokio::net::TcpListener::bind(bind).await?;
            info!(
                "MCP SSE server listening on http://{}",
                listener.local_addr()?
            );
            run_sse_server_with_shutdown(handler.clone(), listener, shutdown_signal).await?;
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

/// Run the SSE (streamable HTTP) transport until the listener fails or
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
async fn run_sse_server_with_shutdown<F>(
    handler: Arc<McpHandler>,
    listener: tokio::net::TcpListener,
    shutdown_signal: F,
) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };

    let bind = listener.local_addr()?;

    let config = StreamableHttpServerConfig::default().with_allowed_hosts(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        bind.ip().to_string(),
        format!("{}:{}", bind.ip(), bind.port()),
    ]);

    let session_manager = Arc::new(LocalSessionManager::default());
    let service_factory = move || Ok(handler.clone());

    let service = StreamableHttpService::new(service_factory, session_manager, config);

    let app = axum::Router::new().fallback_service(service);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| anyhow::anyhow!("axum server error: {e}"))?;

    Ok(())
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
