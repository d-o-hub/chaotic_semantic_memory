use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::args::CliArgs;
use crate::cli::args::CompletionsArgs;
use crate::cli::error::{CliError, Result};

pub fn run_completions(args: CompletionsArgs) -> Result<()> {
    let mut cmd = CliArgs::command();
    let name = cmd.get_name().to_string();

    match args.output {
        Some(path) => write_to_file(&mut cmd, &name, &path, args.shell),
        None => write_to_stdout(&mut cmd, &name, args.shell),
    }
}

fn write_to_stdout(cmd: &mut clap::Command, name: &str, shell: Shell) -> Result<()> {
    clap_complete::generate(shell, cmd, name, &mut std::io::stdout());
    Ok(())
}

fn write_to_file(cmd: &mut clap::Command, name: &str, path: &PathBuf, shell: Shell) -> Result<()> {
    let mut file = File::create(path).map_err(|e| {
        CliError::Output(format!(
            "failed to create output file '{}': {}",
            path.display(),
            e
        ))
    })?;

    clap_complete::generate(shell, cmd, name, &mut file);
    file.flush()
        .map_err(|e| CliError::Io(std::io::Error::new(e.kind(), "failed to flush file")))?;

    eprintln!("Completions written to {}", path.display());
    Ok(())
}
