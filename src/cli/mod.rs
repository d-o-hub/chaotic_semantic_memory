//! CLI implementation for Chaotic Semantic Memory.

pub mod commands;
pub mod args;
pub mod git_local;
pub mod error;

pub use args::{CliArgs, Commands, OutputFormat, CompletionsArgs};
pub use error::{CliError, ExitCode};
pub use git_local::{ensure_git_local_dir, resolve_git_local_path};
pub use commands::*;

use crate::error::Result;
use clap::Parser;

pub async fn run() -> Result<()> {
    let cli = CliArgs::parse();
    let ns = &cli.namespace;

    match cli.command {
        Commands::Inject(args) => commands::inject::run_inject(ns, args).await,
        Commands::Query(args) => commands::query::run_query(ns, args).await,
        Commands::Get(args) => commands::get::run_get(ns, args).await,
        Commands::Delete(args) => commands::delete::run_delete(ns, args).await,
        Commands::Associations(args) => commands::associations::run_associations(ns, args).await,
        Commands::Associate(args) => commands::associate::run_associate(ns, args).await,
        Commands::Disassociate(args) => commands::disassociate::run_disassociate(ns, args).await,
        Commands::Traverse(args) => commands::traverse::run_traverse(ns, args).await,
        Commands::Import(args) => commands::import::run_import(ns, args).await,
        Commands::Export(args) => commands::export::run_export(ns, args).await,
        Commands::Stats(args) => commands::stats::run_stats(ns, args).await,
        Commands::Metrics(args) => commands::metrics::run_metrics(ns, args).await,
        Commands::Watch(args) => commands::watch::run_watch(ns, args).await,
        Commands::ProbeGraph(args) => commands::probe_graph::run_probe_graph(ns, args).await,
        _ => {
            println!("Command not yet implemented in generic CLI");
            Ok(())
        }
    }
}
