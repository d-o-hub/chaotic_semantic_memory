//! Tail subcommand argument structs for the `csm` CLI.
//!
//! Extracted from `crate::cli::args` to stay under the 500-line gate
//! (AGENTS.md rule 9: child-module extraction over comment stripping). The
//! struct bodies are verbatim; `crate::cli::args` re-exports every type here so
//! the existing `crate::cli::args::<Name>Args` paths keep resolving. Mirrors how
//! `crate::cli::mcp` already keeps MCP argument structs out of `args.rs`.

use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct StatsArgs;

#[derive(Args, Debug, Clone)]
pub struct MetricsArgs {
    #[arg(long)]
    pub reset: bool,
}

/// Shared `--ttl-cleanup-interval` definition for the long-running commands.
///
/// Flattened into both `WatchArgs` (`csm watch`) and `McpServeArgs`
/// (`csm mcp serve`) so the flag has exactly one doc comment and one default
/// across the two servers that can hold a background TTL cleanup task.
///
/// The default is deliberately `0` (= disabled): a long-running server must not
/// start deleting expired concepts from the store unless the operator asked for
/// it. Expiry is still filtered at probe time either way, and
/// `ChaoticSemanticFramework::purge_expired` remains the explicit path.
#[derive(Args, Debug, Clone, Copy)]
pub struct TtlCleanupArgs {
    /// Seconds between background TTL cleanup passes; 0 disables the reaper
    /// (default, so nothing is deleted silently). Expired concepts are always
    /// filtered from probe results regardless of this value.
    #[arg(long, value_name = "SECONDS", default_value_t = 0)]
    pub ttl_cleanup_interval: u64,
}

#[derive(Args, Debug, Clone)]
pub struct WatchArgs {
    #[arg(short, long, default_value = "all")]
    pub filter: String,

    /// Background TTL cleanup for this long-running process.
    #[command(flatten)]
    pub ttl_cleanup: TtlCleanupArgs,
}

#[derive(Args, Debug, Clone)]
pub struct ProbeGraphArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub text: String,
    #[arg(long, default_value = "5")]
    pub anchors: usize,
    #[arg(long, default_value = "2")]
    pub hops: usize,
    #[arg(long, default_value = "0.0")]
    pub min_strength: f32,
    #[arg(long, default_value = "0.6")]
    pub similarity_weight: f32,
    #[arg(long, default_value = "0.4")]
    pub graph_weight: f32,
    #[arg(short = 'k', long, default_value = "20")]
    pub top_k: usize,
}

/// Arguments for the history command.
#[derive(Args, Debug, Clone)]
pub struct HistoryArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    /// Show a specific version of the concept.
    #[arg(long, conflicts_with = "rollback")]
    pub version: Option<u64>,
    /// Roll back to a specific version.
    #[arg(long, conflicts_with = "version")]
    pub rollback: Option<u64>,
    /// Skip confirmation prompt for rollback.
    #[arg(short, long, requires = "rollback")]
    pub confirm: bool,
}

/// Arguments for the diff command.
#[derive(Args, Debug, Clone)]
pub struct DiffArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(long)]
    pub from: u64,
    #[arg(long)]
    pub to: u64,
}

/// Arguments for the rollback command.
#[derive(Args, Debug, Clone)]
pub struct RollbackArgs {
    #[arg(long, global = true, default_value = "_default")]
    pub namespace: String,
    #[arg(required = true)]
    pub concept_id: String,
    #[arg(long)]
    pub to: u64,
    #[arg(short, long)]
    pub confirm: bool,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use crate::cli::args::CliArgs;
    use clap::Parser;

    /// Every command goes through `CliArgs`, so the flag is parsed exactly the
    /// way the dispatcher reads it.
    fn watch_ttl_interval(argv: &[&str]) -> u64 {
        let args = CliArgs::try_parse_from(argv).expect("watch should parse");
        match args.command {
            crate::cli::args::Commands::Watch(cmd) => cmd.ttl_cleanup.ttl_cleanup_interval,
            other => panic!("expected Watch, got {other:?}"),
        }
    }

    /// Omitted flag must stay `0` — the reaper may not start silently.
    #[test]
    fn watch_ttl_cleanup_interval_defaults_to_disabled() {
        assert_eq!(watch_ttl_interval(&["csm", "watch"]), 0);
    }

    #[test]
    fn watch_ttl_cleanup_interval_parses_explicit_value() {
        assert_eq!(
            watch_ttl_interval(&["csm", "watch", "--ttl-cleanup-interval", "90"]),
            90
        );
    }

    /// The flattened group must not disturb the flags `watch` already had.
    #[test]
    fn watch_ttl_cleanup_interval_coexists_with_filter() {
        let args = CliArgs::try_parse_from([
            "csm",
            "watch",
            "--filter",
            "deleted",
            "--ttl-cleanup-interval",
            "5",
        ])
        .expect("watch should parse both flags");
        match args.command {
            crate::cli::args::Commands::Watch(cmd) => {
                assert_eq!(cmd.filter, "deleted");
                assert_eq!(cmd.ttl_cleanup.ttl_cleanup_interval, 5);
            }
            other => panic!("expected Watch, got {other:?}"),
        }
    }

    /// A non-numeric value is a usage error, not a silent `0`.
    #[test]
    fn watch_ttl_cleanup_interval_rejects_non_numeric() {
        assert!(
            CliArgs::try_parse_from(["csm", "watch", "--ttl-cleanup-interval", "soon"]).is_err()
        );
    }
}
