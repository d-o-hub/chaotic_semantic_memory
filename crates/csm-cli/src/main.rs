#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
use std::process::ExitCode as StdExitCode;

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
mod native {
    pub use chaotic_semantic_memory::cli::commands::watch::EventFilter;
    pub use chaotic_semantic_memory::cli::{
        CliArgs, CliError, Commands, CompletionsArgs, ExitCode, OutputFormat, ensure_git_local_dir,
        resolve_git_local_path, run_associate, run_associations, run_completions, run_delete,
        run_diff, run_disassociate, run_export, run_get, run_history, run_import, run_index_dir,
        run_index_jsonl, run_inject, run_metrics, run_path, run_probe, run_probe_filtered,
        run_probe_graph, run_query, run_rollback, run_stats, run_traverse, run_update, run_watch,
    };
    pub use clap::Parser;
    pub use colored::Colorize;
    pub use tracing::Level;
    pub use tracing_subscriber::FmtSubscriber;

    /// Environment variable to disable colored output.
    const ENV_NO_COLOR: &str = "NO_COLOR";
    /// Environment variable for target platform.
    const ENV_TARGET: &str = "TARGET";

    pub fn init_tracing(
        verbose: u8,
        _otlp_endpoint: Option<String>,
        _prometheus_bind: Option<String>,
    ) -> Option<Box<dyn std::any::Any>> {
        if std::env::var(ENV_NO_COLOR).is_ok() {
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
                .with_writer(std::io::stderr)
                .finish(),
        );

        None
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
                CliError::Config(format!("Failed to create git-local directory: {e}"))
            })?;
            return Ok(Some(path));
        }

        // Case 4: Default - try git-local storage
        if let Some(path) = resolve_git_local_path() {
            // Found a git repo, use git-local storage by default
            ensure_git_local_dir(&path).map_err(|e| {
                CliError::Config(format!("Failed to create git-local directory: {e}"))
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
                        std::env::var(ENV_TARGET).unwrap_or_else(|_| "unknown".into())
                    );
                }
                Ok(())
            }
            Commands::Inject(cmd) => run_inject(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Probe(cmd) => run_probe(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Query(cmd) => run_query(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Associate(cmd) => run_associate(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Export(cmd) => run_export(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Import(cmd) => run_import(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::IndexJsonl(cmd) => {
                run_index_jsonl(cmd.clone(), db_path.as_deref(), fmt).await
            }
            Commands::IndexDir(cmd) => run_index_dir(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Delete(cmd) => run_delete(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Get(cmd) => run_get(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::History(cmd) => run_history(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Diff(cmd) => run_diff(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Rollback(cmd) => run_rollback(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Update(cmd) => run_update(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Disassociate(cmd) => {
                run_disassociate(cmd.clone(), db_path.as_deref(), fmt).await
            }
            Commands::Associations(cmd) => {
                run_associations(cmd.clone(), db_path.as_deref(), fmt).await
            }
            Commands::Traverse(cmd) => run_traverse(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::Path(cmd) => run_path(cmd.clone(), db_path.as_deref(), fmt).await,
            Commands::ProbeFiltered(cmd) => {
                run_probe_filtered(cmd.clone(), db_path.as_deref(), fmt).await
            }
            Commands::Stats(_cmd) => run_stats(db_path.as_deref(), fmt).await,
            Commands::Metrics(cmd) => run_metrics(db_path.as_deref(), fmt, cmd.reset).await,
            Commands::Watch(cmd) => {
                let filter = EventFilter::parse(&cmd.filter).ok_or_else(|| {
                    CliError::Validation(format!(
                        "invalid filter '{}': use all, injected, updated, deleted, associated, or disassociated",
                        cmd.filter
                    ))
                })?;
                run_watch(db_path.as_deref(), filter).await
            }
            Commands::ProbeGraph(cmd) => {
                run_probe_graph(cmd.clone(), db_path.as_deref(), fmt).await
            }
            #[cfg(feature = "mcp")]
            Commands::Mcp(cmd) => match cmd {
                chaotic_semantic_memory::cli::McpCommands::Serve(args) => {
                    let config = chaotic_semantic_memory::mcp::McpConfig {
                        transport: match args.transport { chaotic_semantic_memory::mcp::TransportType::Stdio => chaotic_semantic_memory::mcp::Transport::Stdio, chaotic_semantic_memory::mcp::TransportType::Sse => chaotic_semantic_memory::mcp::Transport::Sse { bind: args.bind.clone().unwrap_or_else(|| "127.0.0.1:3000".to_string()).parse().expect("Invalid address") } },
                        bind: args.bind.clone(),
                        database: db_path,
                    };
                    chaotic_semantic_memory::mcp::serve(config)
                        .await
                        .map_err(|e| {
                            chaotic_semantic_memory::cli::CliError::Persistence(e.to_string())
                        })
                }
            },
        };
        result.map(|_| ((), fmt))
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
fn main() -> StdExitCode {
    use native::*;
    let args = CliArgs::parse();
    let output_format = args.output_format;
    init_tracing(args.verbose, None, None);
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
