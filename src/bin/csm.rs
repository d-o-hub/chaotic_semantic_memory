#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
use std::process::ExitCode as StdExitCode;

#[cfg(all(not(target_arch = "wasm32"), feature = "cli"))]
mod native {
    pub use chaotic_semantic_memory::cli::{
        CliArgs, CliError, Commands, CompletionsArgs, ExitCode, OutputFormat, run_associate,
        run_completions, run_export, run_import, run_inject, run_probe,
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

    #[tokio::main]
    pub async fn run_async(args: CliArgs) -> Result<((), OutputFormat), CliError> {
        let db_path = args.database.as_deref();
        let fmt = args.output_format;

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
            Commands::Inject(cmd) => run_inject(cmd.clone(), db_path, fmt).await,
            Commands::Probe(cmd) => run_probe(cmd.clone(), db_path, fmt).await,
            Commands::Associate(cmd) => run_associate(cmd.clone(), db_path, fmt).await,
            Commands::Export(cmd) => run_export(cmd.clone(), db_path, fmt).await,
            Commands::Import(cmd) => run_import(cmd.clone(), db_path, fmt).await,
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
