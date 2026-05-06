//! Watch command for real-time event streaming.

use std::io::{self, BufWriter, Write};
use std::path::Path;

use tracing::instrument;

use crate::cli::error::{CliError, Result};
use crate::framework_events::MemoryEvent;

use super::create_framework;

/// Filter for event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFilter {
    All,
    ConceptInjected,
    ConceptUpdated,
    ConceptDeleted,
    Associated,
    Disassociated,
}

impl EventFilter {
    /// Check if the given event matches this filter.
    pub const fn matches(&self, event: &MemoryEvent) -> bool {
        match self {
            EventFilter::All => true,
            EventFilter::ConceptInjected => {
                matches!(event, MemoryEvent::ConceptInjected { .. })
            }
            EventFilter::ConceptUpdated => {
                matches!(event, MemoryEvent::ConceptUpdated { .. })
            }
            EventFilter::ConceptDeleted => {
                matches!(event, MemoryEvent::ConceptDeleted { .. })
            }
            EventFilter::Associated => {
                matches!(event, MemoryEvent::Associated { .. })
            }
            EventFilter::Disassociated => {
                matches!(event, MemoryEvent::Disassociated { .. })
            }
        }
    }

    /// Parse from string (for clap).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "all" => Some(EventFilter::All),
            "injected" | "concept_injected" => Some(EventFilter::ConceptInjected),
            "updated" | "concept_updated" => Some(EventFilter::ConceptUpdated),
            "deleted" | "concept_deleted" => Some(EventFilter::ConceptDeleted),
            "associated" => Some(EventFilter::Associated),
            "disassociated" => Some(EventFilter::Disassociated),
            _ => None,
        }
    }
}

/// Run the watch command.
#[instrument(name = "cli_watch")]
pub async fn run_watch(db_path: Option<&Path>, filter: EventFilter) -> Result<()> {
    let framework: crate::framework::ChaoticSemanticFramework = create_framework(db_path).await?;
    let mut receiver = framework.subscribe();

    // Use buffered stdout for efficient line-by-line output
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    // Print a startup message to stderr so it doesn't interfere with JSONL output
    eprintln!("Watching for memory events (filter: {:?})...", filter);
    eprintln!("Press Ctrl+C to stop.");

    loop {
        match receiver.recv().await {
            Ok(event) => {
                if filter.matches(&event) {
                    let json = event_to_json(&event);
                    writeln!(writer, "{json}")
                        .map_err(|e| CliError::Output(format!("failed to write event: {e}")))?;
                    writer
                        .flush()
                        .map_err(|e| CliError::Output(format!("failed to flush output: {e}")))?;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                // Channel closed, exit gracefully
                eprintln!("Event channel closed.");
                break;
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                // Lagged behind - warn but continue
                eprintln!("Warning: Lagged {n} events, continuing...");
            }
        }
    }

    Ok(())
}

/// Convert a MemoryEvent to a JSON string for JSONL output.
fn event_to_json(event: &MemoryEvent) -> String {
    match event {
        MemoryEvent::ConceptInjected { id, timestamp } => serde_json::json!({
            "event": "concept_injected",
            "id": id,
            "timestamp": timestamp
        })
        .to_string(),
        MemoryEvent::ConceptUpdated { id, timestamp } => serde_json::json!({
            "event": "concept_updated",
            "id": id,
            "timestamp": timestamp
        })
        .to_string(),
        MemoryEvent::ConceptDeleted { id, timestamp } => serde_json::json!({
            "event": "concept_deleted",
            "id": id,
            "timestamp": timestamp
        })
        .to_string(),
        MemoryEvent::Associated { from, to, strength } => serde_json::json!({
            "event": "associated",
            "from": from,
            "to": to,
            "strength": strength
        })
        .to_string(),
        MemoryEvent::Disassociated { from, to } => serde_json::json!({
            "event": "disassociated",
            "from": from,
            "to": to
        })
        .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_matches() {
        let injected = MemoryEvent::ConceptInjected {
            id: "test".to_string(),
            timestamp: 0,
        };
        let deleted = MemoryEvent::ConceptDeleted {
            id: "test".to_string(),
            timestamp: 0,
        };
        let associated = MemoryEvent::Associated {
            from: "a".to_string(),
            to: "b".to_string(),
            strength: 0.5,
        };

        // All filter matches everything
        assert!(EventFilter::All.matches(&injected));
        assert!(EventFilter::All.matches(&deleted));
        assert!(EventFilter::All.matches(&associated));

        // Specific filters
        assert!(EventFilter::ConceptInjected.matches(&injected));
        assert!(!EventFilter::ConceptInjected.matches(&deleted));
        assert!(EventFilter::ConceptDeleted.matches(&deleted));
        assert!(EventFilter::Associated.matches(&associated));
    }

    #[test]
    fn test_event_filter_parse() {
        assert_eq!(EventFilter::parse("all"), Some(EventFilter::All));
        assert_eq!(
            EventFilter::parse("injected"),
            Some(EventFilter::ConceptInjected)
        );
        assert_eq!(
            EventFilter::parse("CONCEPT_INJECTED"),
            Some(EventFilter::ConceptInjected)
        );
        assert_eq!(
            EventFilter::parse("deleted"),
            Some(EventFilter::ConceptDeleted)
        );
        assert_eq!(
            EventFilter::parse("associated"),
            Some(EventFilter::Associated)
        );
        assert_eq!(EventFilter::parse("unknown"), None);
    }

    #[test]
    fn test_event_to_json_format() {
        let event = MemoryEvent::ConceptInjected {
            id: "test-id".to_string(),
            timestamp: 12345,
        };
        let json = event_to_json(&event);
        assert!(json.contains("\"event\":\"concept_injected\""));
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"timestamp\":12345"));
    }
}
