//! MCP CLI command definitions (ADR-0067)
use clap::{Args, Subcommand};

#[cfg(feature = "mcp")]
#[derive(Subcommand, Debug, Clone)]
pub enum McpCommands {
    /// Start MCP server.
    Serve(McpServeArgs),
}

#[cfg(feature = "mcp")]
#[derive(Args, Debug, Clone)]
pub struct McpServeArgs {
    /// Transport to use: stdio, sse.
    #[arg(long, value_enum, default_value = "stdio")]
    pub transport: crate::mcp::TransportType,

    /// Bind address for SSE transport (e.g. 127.0.0.1:8765).
    #[arg(long)]
    pub bind: Option<String>,
}
