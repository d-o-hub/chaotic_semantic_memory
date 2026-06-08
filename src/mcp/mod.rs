//! MCP Server module (ADR-0067)
//!
//! Provides Model Context Protocol server for AI agent integration.
//! Enabled via `mcp` feature flag.

#[cfg(feature = "mcp")]
mod handler;
#[cfg(feature = "mcp")]
mod schema;
#[cfg(feature = "mcp")]
mod server;

#[cfg(feature = "mcp")]
pub use handler::McpHandler;
#[cfg(feature = "mcp")]
pub use handler::parse_hvec;
#[cfg(feature = "mcp")]
pub use server::{McpConfig, Transport, serve};
