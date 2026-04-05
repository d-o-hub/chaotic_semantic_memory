#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
use std::process::ExitCode as StdExitCode;

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
mod native {
    pub use chaotic_semantic_memory::cli::{
        CliArgs, CliError, Commands, CompletionsArgs, ExitCode, OutputFormat, ensure_git_local_dir,
        resolve_git_local_path, run_associate, run_completions, run_export, run_import,
        run_index_dir, run_index_jsonl, run_inject, run_probe, run_query,
    };
    pub use clap::Parser;
    pub use colored::Colorize;
    pub use tracing::Level;
    pub use tracing_subscriber::FmtSubscriber;

    pub fn init_tracing(verbose: u8) {
        if std::env::var("NO_COLOR").is_ok() {
            colored::control::set_override(false);
        }
        let level = match verbose {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        };
        let _ = tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(level)
                .with_target(false)
                .finish(),
        );
    }

    pub fn format_error(err: &CliError, format: OutputFormat) -> String {
        match format {
            OutputFormat::Json => {
                serde_json::json!({"status": "error", "error": err.to_string()}).to_string()
            }
            _ => format!("{}: {}", "error".red().bold(), err),
        }
    }

    pub fn handle_completions(args: &CompletionsArgs) -> Result<(), CliError> {
        run_completions(args.clone())
    }

    /// Resolve database path from CLI arguments.
    ///
    /// Priority order:
    /// 1. Explicit --database path (if provided)
    /// 2. Explicit --index-path (if provided with --git-local)
    /// 3. Git-local storage (.git/memory-index/csm.db) if in git repo and no --database
    /// 4. None (in-memory mode) if not in git repo and no --database
    pub fn resolve_database_path(
        database: Option<&std::path::Path>,
        git_local: bool,
        index_path: Option<&std::path::Path>,
    ) -> Result<Option<std::path::PathBuf>, CliError> {
        // Case 1: Explicit --database path provided
        if let Some(db_path) = database {
            return Ok(Some(db_path.to_path_buf()));
        }

        // Case 2: --index-path override with --git-local
        if let Some(custom_path) = index_path {
            if !git_local {
                return Err(CliError::Config(
                    "--index-path requires --git-local to be specified".to_string(),
                ));
            }
            return Ok(Some(custom_path.to_path_buf()));
        }

        // Case 3: --git-local explicitly requested
        if git_local {
            let path = resolve_git_local_path().ok_or_else(|| {
                CliError::Config(
                    "--git-local specified but not in a git repository. \
                     Run this command inside a git repo or use --database to specify a path."
                        .to_string(),
                )
            })?;
            ensure_git_local_dir(&path).map_err(|e| {
                CliError::Config(format!("Failed to create git-local directory: {}", e))
            })?;
            return Ok(Some(path));
        }

        // Case 4: Default - try git-local storage
        if let Some(path) = resolve_git_local_path() {
            // Found a git repo, use git-local storage by default
            ensure_git_local_dir(&path).map_err(|e| {
                CliError::Config(format!("Failed to create git-local directory: {}", e))
            })?;
            return Ok(Some(path));
        }

        // Case 5: Not in git repo and no database specified - use in-memory mode
        Ok(None)
    }

    #[tokio::main]
    pub async fn run_async(args: CliArgs) -> Result<((), OutputFormat), CliError> {
        let fmt = args.output_format;

        // Resolve the database path with git-local support
        let db_path = resolve_database_path(
            args.database.as_deref(),
            args.git_local,
            args.index_path.as_deref(),
        )?;

        let result = match &args.command {
            Commands::Completions(cmd) => handle_completions(cmd),
            Commands::Version(v) => {
                println!("csm {}", env!("CARGO_PKG_VERSION"));
                if v.detailed {
                    println!(
                        "target: {}",
                        std::env::var("TARGET").unwrap_or_else(|_| "unknown".into())
                    );
                }
                Ok(())
            }
            Commands::Inject(cmd) => run_inject(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Probe(cmd) => run_probe(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Query(cmd) => {
                run_query(
                    cmd.clone(),
                    db_path.as_deref(),
                    args.git_local,
                    args.index_path.as_deref(),
                    fmt,
                )
                .await
            }
            Commands::Associate(cmd) => run_associate(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Export(cmd) => run_export(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Import(cmd) => run_import(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::IndexJsonl(cmd) => {
                run_index_jsonl(
                    cmd.clone(),
                    db_path.as_deref(),
                    args.git_local,
                    args.index_path.as_deref(),
                    fmt,
                )
                .await
            }
            Commands::IndexDir(cmd) => {
                run_index_dir(
                    cmd.clone(),
                    db_path.as_deref(),
                    args.git_local,
                    args.index_path.as_deref(),
                    fmt,
                )
                .await
            }
        };
        result.map(|_| ((), fmt))
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
fn main() -> StdExitCode {
    use native::*;
    let args = CliArgs::parse();
    let output_format = args.output_format;
    init_tracing(args.verbose);
    match run_async(args) {
        Ok(_) => StdExitCode::from(ExitCode::Success as u8),
        Err(ref e) => {
            eprintln!("{}", format_error(e, output_format));
            StdExitCode::from(ExitCode::from(e) as u8)
        }
    }
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "cli")))]
fn main() {
    eprintln!("CLI is not available. Build with --features cli to enable.");
    std::process::exit(1);
}
