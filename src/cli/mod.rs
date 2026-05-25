pub mod args;
pub mod commands;
pub mod error;
pub mod git_local;
#[cfg(feature = "mcp")]
pub mod mcp;

pub use args::*;
pub use commands::{
    run_associate, run_associations, run_completions, run_delete, run_diff, run_disassociate,
    run_export, run_get, run_history, run_import, run_index_dir, run_index_jsonl, run_inject,
    run_metrics, run_path, run_probe, run_probe_filtered, run_probe_graph, run_query, run_rollback,
    run_stats, run_traverse, run_update, run_watch,
};
pub use error::{CliError, ExitCode, Result};
pub use git_local::{ensure_git_local_dir, resolve_git_local_path};
#[cfg(feature = "mcp")]
pub use mcp::*;
