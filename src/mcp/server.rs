//! MCP Server implementation using rmcp (ADR-0067)
//!
//! Provides stdio and SSE transports for Claude Desktop, Cursor, and other MCP clients.

use anyhow::Result;
use tracing::info;

use crate::mcp::handler::McpHandler;

/// Transport type for MCP server.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum Transport {
    /// Standard input/output (default for desktop apps)
    #[default]
    Stdio,
    /// Server-Sent Events for hosted deployments
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
        Transport::Sse => {
            return Err(anyhow::anyhow!(
                "SSE transport not yet fully implemented in this version"
            ));
        }
    }

    Ok(())
}
