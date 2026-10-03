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

#[derive(Args, Debug, Clone)]
pub struct WatchArgs {
    #[arg(short, long, default_value = "all")]
    pub filter: String,
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

