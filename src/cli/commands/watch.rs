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
                // ADR-0066 says: csm watch streams JSONL (one event per line, flushed per write).
                // Honor filter if provided
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

                let json = match event {
                    crate::framework_events::MemoryEvent::ConceptInjected { id, timestamp } => {
                        serde_json::json!({"event": "ConceptInjected", "id": id, "timestamp": timestamp})
                    }
                    crate::framework_events::MemoryEvent::ConceptUpdated { id, timestamp } => {
                        serde_json::json!({"event": "ConceptUpdated", "id": id, "timestamp": timestamp})
                    }
                    crate::framework_events::MemoryEvent::ConceptDeleted { id, timestamp } => {
                        serde_json::json!({"event": "ConceptDeleted", "id": id, "timestamp": timestamp})
                    }
                    crate::framework_events::MemoryEvent::Associated { from, to, strength } => {
                        serde_json::json!({"event": "Associated", "from": from, "to": to, "strength": strength})
                    }
                    crate::framework_events::MemoryEvent::Disassociated { from, to } => {
                        serde_json::json!({"event": "Disassociated", "from": from, "to": to})
                    }
                };

                let mut line = serde_json::to_vec(&json).unwrap();
                line.push(b'\n');
                if let Err(e) = stdout.write_all(&line).await {
                    return Err(CliError::Io(e));
                }
                if let Err(e) = stdout.flush().await {
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
