pub mod resources;
pub mod schema;
pub mod server;
pub mod tools;

use crate::cli::args::McpTransport;
use crate::framework::ChaoticSemanticFramework;

/// Start the MCP server with the given framework and transport.
pub async fn serve(
    framework: ChaoticSemanticFramework,
    transport: McpTransport,
    bind: &str,
) -> crate::error::Result<()> {
    let server = server::McpServer::new(framework);

    match transport {
        McpTransport::Stdio => {
            server
                .run_stdio()
                .await
                .map_err(|e| crate::error::MemoryError::internal_error(e.to_string()))?;
        }
        McpTransport::Sse => {
            server
                .run_sse(bind)
                .await
                .map_err(|e| crate::error::MemoryError::internal_error(e.to_string()))?;
        }
    }

    Ok(())
}
