use crate::framework::ChaoticSemanticFramework;
use crate::mcp::server::McpServer;

pub mod resources;
pub mod schema;
pub mod server;
pub mod tools;

pub async fn serve(
    framework: ChaoticSemanticFramework,
    transport: &str,
    bind: &str,
) -> crate::error::Result<()> {
    let server = McpServer::new(framework);

    match transport {
        "stdio" => {
            server.run_stdio().await.map_err(|e| {
                crate::error::MemoryError::database(format!("MCP server error: {}", e.message))
            })?;
        }
        "sse" => {
            server.run_sse(bind).await.map_err(|e| {
                crate::error::MemoryError::database(format!("MCP server error: {}", e.message))
            })?;
        }
        _ => {
            return Err(crate::error::MemoryError::database(format!(
                "Unsupported transport: {}",
                transport
            )));
        }
    }

    Ok(())
}
