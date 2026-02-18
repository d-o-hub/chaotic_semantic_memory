use std::process::ExitCode as StdExitCode;

use chaotic_semantic_memory::cli::{
    run_associate, run_completions, run_export, run_import, run_inject, run_probe, CliArgs,
    CliError, Commands, CompletionsArgs, ExitCode, OutputFormat,
};
use clap::Parser;
use colored::Colorize;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn init_tracing(verbose: u8) {
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

fn format_error(err: &CliError, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => {
            serde_json::json!({"status": "error", "error": err.to_string()}).to_string()
        }
        _ => format!("{}: {}", "error".red().bold(), err),
    }
}

fn handle_completions(args: &CompletionsArgs) -> Result<(), CliError> {
    run_completions(args.clone()).map_err(CliError::from)
}

#[tokio::main]
async fn run_async(args: CliArgs) -> Result<(), CliError> {
    let db_path = args.database.as_deref();
    let fmt = args.output_format;

    match &args.command {
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
        Commands::Inject(cmd) => run_inject(cmd.clone(), db_path, fmt)
            .await
            .map_err(CliError::from),
        Commands::Probe(cmd) => run_probe(cmd.clone(), db_path, fmt)
            .await
            .map_err(CliError::from),
        Commands::Associate(cmd) => run_associate(cmd.clone(), db_path, fmt)
            .await
            .map_err(CliError::from),
        Commands::Export(cmd) => run_export(cmd.clone(), db_path, fmt)
            .await
            .map_err(CliError::from),
        Commands::Import(cmd) => run_import(cmd.clone(), db_path, fmt)
            .await
            .map_err(CliError::from),
    }
}

fn main() -> StdExitCode {
    let args = CliArgs::parse();
    init_tracing(args.verbose);
    match run_async(args) {
        Ok(()) => StdExitCode::from(ExitCode::Success as u8),
        Err(ref e) => {
            eprintln!("{}", format_error(e, OutputFormat::Table));
            StdExitCode::from(ExitCode::from(e) as u8)
        }
    }
}
