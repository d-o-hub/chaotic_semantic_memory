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
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            bind: None,
            database: None,
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

    let handler = McpHandler::new(config.database);

    match config.transport {
        Transport::Stdio => {
            let (stdin, stdout) = rmcp::transport::io::stdio();
            let server = rmcp::serve_server(handler, (stdin, stdout)).await?;
            server
                .waiting()
                .await
                .map_err(|e| anyhow::anyhow!("Server join error: {e}"))?;
        }
        Transport::Sse { bind } => {
            run_sse_server(handler, bind).await?;
        }
    }

    Ok(())
}

async fn run_sse_server(handler: McpHandler, bind: std::net::SocketAddr) -> Result<()> {
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

impl Clone for McpHandler {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            framework: tokio::sync::OnceCell::new(), // New instance will re-init
        }
    }
}
