use crate::cli::args::{McpArgs, McpCommands, McpTransport};
use crate::cli::commands::create_framework;
use crate::cli::error::Result;


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
                .map_err(|e| crate::cli::error::CliError::Persistence(e.to_string()))
        }
    }
}
