//! MCP Server implementation using rmcp (ADR-0067)
//!
//! Provides stdio and SSE transports for Claude Desktop, Cursor, and other MCP clients.

use anyhow::Result;
use tracing::info;

use super::resources::McpResources;
use super::tools::McpTools;

/// Transport type for MCP server.
#[derive(Debug, Clone, Copy, Default)]
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

    // TODO: Wire up rmcp server with tools and resources
    // This is a stub implementation - full rmcp integration requires:
    // 1. ServerBuilder with tool/resource registration
    // 2. Transport selection (stdio vs SSE)
    // 3. Running the server loop

    match config.transport {
        Transport::Stdio => {
            // Stdio transport for Claude Desktop / Cursor
            serve_stdio(config).await?;
        }
        Transport::Sse => {
            // SSE transport for hosted deployments
            let bind = config
                .bind
                .clone()
                .unwrap_or_else(|| "127.0.0.1:3030".to_string());
            serve_sse(config, &bind).await?;
        }
    }

    Ok(())
}

async fn serve_stdio(config: McpConfig) -> Result<()> {
    let _tools = McpTools::new(config.database);
    let _resources = McpResources::new(None); // TODO: use config.database when wired

    // Stub: actual rmcp integration would be:
    // let server = rmcp::ServerBuilder::new()
    //     .add_tools(&tools)
    //     .add_resources(&resources)
    //     .stdio_transport()
    //     .serve()
    //     .await?;

    info!("MCP stdio server ready");
    Ok(())
}

async fn serve_sse(config: McpConfig, bind: &str) -> Result<()> {
    let _tools = McpTools::new(config.database);
    let _resources = McpResources::new(None); // TODO: use config.database when wired

    // Stub: actual rmcp integration would be:
    // let server = rmcp::ServerBuilder::new()
    //     .add_tools(&tools)
    //     .add_resources(&resources)
    //     .sse_transport(bind)
    //     .serve()
    //     .await?;

    info!("MCP SSE server listening on {}", bind);
    Ok(())
}
