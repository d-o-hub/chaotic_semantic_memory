use std::path::Path;
use tracing::instrument;
use crate::cli::args::{McpArgs, McpCommands};
use crate::cli::error::{CliError, Result};
use super::create_framework;

#[instrument(name = "cli_mcp", skip(args))]
pub async fn run_mcp(
    args: McpArgs,
    db_path: Option<&Path>,
) -> Result<()> {
    match args.command {
        McpCommands::Serve(serve_args) => {
            let framework = create_framework(db_path).await?;

            #[cfg(feature = "mcp")]
            {
                crate::mcp::serve(framework, serve_args.transport, &serve_args.bind).await
                    .map_err(|e| CliError::Persistence(format!("MCP server error: {}", e)))?;
            }

            #[cfg(not(feature = "mcp"))]
            {
                return Err(CliError::Config("MCP feature is not enabled. Build with --features mcp to use this command.".to_string()));
            }
        }
    }

    Ok(())
}
