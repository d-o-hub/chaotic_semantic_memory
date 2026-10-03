//! MCP Server implementation using rmcp (ADR-0067)
//!
//! Provides stdio and SSE transports for Claude Desktop, Cursor, and other MCP clients.

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
/// # Errors
///
/// Returns error if server fails to start or transport initialization fails.
pub async fn serve(config: McpConfig) -> Result<()> {
    info!("Starting MCP server with {:?} transport", config.transport);

    // The reaper interval is carried on the handler, not the transport, because
    // the framework is created lazily on the first tool call — this is the only
    // place that knows both the config value and the handler that builds it.
    let handler = Arc::new(
        McpHandler::new(config.database).with_ttl_cleanup_interval(config.ttl_cleanup_interval),
    );

    match config.transport {
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
            run_sse_server(handler.clone(), bind).await?;
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

async fn run_sse_server(handler: Arc<McpHandler>, bind: std::net::SocketAddr) -> Result<()> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };

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

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("MCP SSE server listening on http://{}", bind);
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("axum server error: {e}"))?;

    Ok(())
}
