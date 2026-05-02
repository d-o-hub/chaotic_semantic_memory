use super::create_framework;
use crate::cli::args::{OutputFormat, WatchArgs};
use crate::cli::error::{CliError, Result};
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::instrument;

#[instrument(name = "cli_watch")]
pub async fn run_watch(
    args: WatchArgs,
    db_path: Option<&Path>,
    _format: OutputFormat,
) -> Result<()> {
    let framework = create_framework(db_path).await?;
    let mut rx = framework.subscribe();

    let mut stdout = tokio::io::stdout();

    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Some(ref filter_kind) = args.filter {
                    let kind = match event {
                        crate::framework_events::MemoryEvent::ConceptInjected { .. } => {
                            "ConceptInjected"
                        }
                        crate::framework_events::MemoryEvent::ConceptUpdated { .. } => {
                            "ConceptUpdated"
                        }
                        crate::framework_events::MemoryEvent::ConceptDeleted { .. } => {
                            "ConceptDeleted"
                        }
                        crate::framework_events::MemoryEvent::Associated { .. } => "Associated",
                        crate::framework_events::MemoryEvent::Disassociated { .. } => {
                            "Disassociated"
                        }
                    };
                    if kind != filter_kind {
                        continue;
                    }
                }

                let mut line = serde_json::to_vec(&event).unwrap();
                line.push(b'\n');
                if let Err(e) = stdout.write_all(&line).await {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        return Ok(());
                    }
                    return Err(CliError::Io(e));
                }
                if let Err(e) = stdout.flush().await {
                    if e.kind() == std::io::ErrorKind::BrokenPipe {
                        return Ok(());
                    }
                    return Err(CliError::Io(e));
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("Warning: Watch lagged by {} events", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }

    Ok(())
}
