use crate::cli::args::McpArgs;
#[cfg(feature = "mcp")]
use crate::cli::args::{McpCommands, McpTransport};
#[cfg(feature = "mcp")]
use crate::cli::commands::create_framework;
use crate::cli::error::{CliError, Result};

#[cfg(feature = "mcp")]
pub async fn run_mcp(
    args: McpArgs,
    db_path: Option<&std::path::Path>,
) -> Result<()> {
    let framework = create_framework(db_path).await?;

    match args.command {
        McpCommands::Serve(serve_args) => {
            let transport_str = match serve_args.transport {
                McpTransport::Stdio => "stdio",
                McpTransport::Sse => "sse",
            };
            crate::mcp::serve(framework, transport_str, &serve_args.bind).await
                .map_err(|e| CliError::Persistence(e.to_string()))
        }
    }
}

#[cfg(not(feature = "mcp"))]
pub async fn run_mcp(
    _args: McpArgs,
    _db_path: Option<&std::path::Path>,
) -> Result<()> {
    Err(CliError::Persistence("MCP feature is not enabled in this build. Recompile with --features mcp".into()))
}
