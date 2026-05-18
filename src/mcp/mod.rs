//! MCP Server module (ADR-0067)
//!
//! Provides Model Context Protocol server for AI agent integration.
//! Enabled via `mcp` feature flag.

#[cfg(feature = "mcp")]
mod resources;
#[cfg(feature = "mcp")]
mod schema;
#[cfg(feature = "mcp")]
mod server;
#[cfg(feature = "mcp")]
mod tools;

#[cfg(feature = "mcp")]
pub use resources::McpResources;
#[cfg(feature = "mcp")]
pub use server::{Transport, McpConfig, serve};
#[cfg(feature = "mcp")]
pub use tools::McpTools;
