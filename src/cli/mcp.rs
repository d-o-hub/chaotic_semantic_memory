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

    /// Background TTL cleanup for this long-running server (0 = disabled).
    ///
    /// Shares `TtlCleanupArgs` with `csm watch` so both servers parse the same
    /// flag with the same default; the value reaches the framework through
    /// `McpConfig::ttl_cleanup_interval`.
    #[command(flatten)]
    pub ttl_cleanup: crate::cli::args_commands::TtlCleanupArgs,
}

#[cfg(all(test, feature = "mcp"))]
mod tests {
    use super::*;
    use crate::cli::args::CliArgs;
    use clap::Parser;

    fn serve_ttl_interval(argv: &[&str]) -> u64 {
        let args = CliArgs::try_parse_from(argv).expect("mcp serve should parse");
        match args.command {
            crate::cli::args::Commands::Mcp(McpCommands::Serve(cmd)) => {
                cmd.ttl_cleanup.ttl_cleanup_interval
            }
            other => panic!("expected Mcp::Serve, got {other:?}"),
        }
    }

    /// `mcp serve` without the flag must not start a reaper.
    #[test]
    fn serve_ttl_cleanup_interval_defaults_to_disabled() {
        assert_eq!(serve_ttl_interval(&["csm", "mcp", "serve"]), 0);
    }

    #[test]
    fn serve_ttl_cleanup_interval_parses_explicit_value() {
        assert_eq!(
            serve_ttl_interval(&["csm", "mcp", "serve", "--ttl-cleanup-interval", "45"]),
            45
        );
    }

    /// The flattened group must not disturb the transport flags.
    #[test]
    fn serve_ttl_cleanup_interval_coexists_with_transport() {
        let args = CliArgs::try_parse_from([
            "csm",
            "mcp",
            "serve",
            "--transport",
            "sse",
            "--bind",
            "127.0.0.1:8765",
            "--ttl-cleanup-interval",
            "7",
        ])
        .expect("mcp serve should parse every flag");
        match args.command {
            crate::cli::args::Commands::Mcp(McpCommands::Serve(cmd)) => {
                assert!(matches!(cmd.transport, crate::mcp::TransportType::Sse));
                assert_eq!(cmd.bind.as_deref(), Some("127.0.0.1:8765"));
                assert_eq!(cmd.ttl_cleanup.ttl_cleanup_interval, 7);
            }
            other => panic!("expected Mcp::Serve, got {other:?}"),
        }
    }
}
