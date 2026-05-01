pub mod args;
pub mod commands;
pub mod error;
pub mod git_local;

pub use args::*;
pub use commands::{
    run_associate, run_completions, run_export, run_import, run_index_dir, run_index_jsonl,
    run_inject, run_mcp, run_probe, run_query,
};
pub use error::{CliError, ExitCode, Result};
pub use git_local::{ensure_git_local_dir, resolve_git_local_path};
