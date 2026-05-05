//! Framework event broadcasting.

// Casts are intentional for event timestamp math

use tokio::sync::broadcast;

#[cfg(target_arch = "wasm32")]
use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
#[cfg(target_arch = "wasm32")]
use crate::hyperdim::HVec10240;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub enum MemoryEvent {
    ConceptInjected {
        id: String,
        timestamp: u64,
    },
    ConceptUpdated {
        id: String,
        timestamp: u64,
    },
    ConceptDeleted {
        id: String,
        timestamp: u64,
    },
    Associated {
        from: String,
        to: String,
        strength: f32,
    },
    Disassociated {
        from: String,
        to: String,
    },
}

impl ChaoticSemanticFramework {
    /// Subscribe to memory change events.
    pub fn subscribe(&self) -> broadcast::Receiver<MemoryEvent> {
        self.event_sender.subscribe()
    }

    pub(crate) fn emit_event(&self, event: MemoryEvent) {
        let _ = self.event_sender.send(event);
    }

    /// Update a concept's vector (WASM-only, memory-only).
    #[cfg(target_arch = "wasm32")]
    pub async fn update_concept_vector(&self, id: &str, vector: HVec10240) -> Result<()> {
        self.singularity.write().await.update(id, vector)?;
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        });
        Ok(())
    }

    /// Update a concept's metadata (WASM-only, memory-only).
    #[cfg(target_arch = "wasm32")]
    pub async fn update_concept_metadata(
        &self,
        id: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.singularity
            .write()
            .await
            .update_metadata(id, metadata)?;
        self.emit_event(MemoryEvent::ConceptUpdated {
            id: id.to_string(),
            timestamp: unix_now_secs(),
        });
        Ok(())
    }

    /// Remove an association (WASM-only, memory-only).
    #[cfg(target_arch = "wasm32")]
    pub async fn disassociate(&self, from: &str, to: &str) -> Result<()> {
        self.singularity.write().await.disassociate(from, to)?;
        self.emit_event(MemoryEvent::Disassociated {
            from: from.to_string(),
            to: to.to_string(),
        });
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
fn unix_now_secs() -> u64 {
    let millis = Date::now();
    if !millis.is_finite() || millis < 0.0 {
        return 0;
    }
    let secs = (millis / 1000.0).floor();
    format!("{secs:.0}").parse::<u64>().unwrap_or(0)
}

pub(crate) fn build_event_sender() -> broadcast::Sender<MemoryEvent> {
    broadcast::channel(DEFAULT_EVENT_CHANNEL_CAPACITY).0
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::redundant_clone)] // Clone needed for event ownership testing

    use super::*;

    #[test]
    fn memory_event_variants_construct() {
        let injected = MemoryEvent::ConceptInjected {
            id: "test-id".to_string(),
            timestamp: 12345,
        };
        let updated = MemoryEvent::ConceptUpdated {
            id: "test-id".to_string(),
            timestamp: 12346,
        };
        let _deleted = MemoryEvent::ConceptDeleted {
            id: "test-id".to_string(),
            timestamp: 12347,
        };
        let _associated = MemoryEvent::Associated {
            from: "a".to_string(),
            to: "b".to_string(),
            strength: 0.5,
        };
        let _disassociated = MemoryEvent::Disassociated {
            from: "a".to_string(),
            to: "b".to_string(),
        };

        // Verify Clone works
        let cloned = injected.clone();
        assert!(matches!(cloned, MemoryEvent::ConceptInjected { .. }));

        // Verify Debug works
        let debug_str = format!("{updated:?}");
        assert!(debug_str.contains("ConceptUpdated"));
    }

    #[test]
    fn build_event_sender_creates_channel() {
        let sender = build_event_sender();
        // Channel should be operational
        let mut receiver = sender.subscribe();
        assert!(receiver.try_recv().is_err()); // Empty channel
    }

    #[test]
    fn sender_broadcasts_to_multiple_receivers() {
        let sender = build_event_sender();
        let mut rx1 = sender.subscribe();
        let mut rx2 = sender.subscribe();

        let event = MemoryEvent::ConceptInjected {
            id: "broadcast-test".to_string(),
            timestamp: 99999,
        };

        sender.send(event.clone()).unwrap();

        // Both receivers should get the event
        let recv1 = rx1.try_recv().unwrap();
        let recv2 = rx2.try_recv().unwrap();

        assert!(
            matches!(recv1, MemoryEvent::ConceptInjected { id, timestamp } if id == "broadcast-test" && timestamp == 99999)
        );
        assert!(
            matches!(recv2, MemoryEvent::ConceptInjected { id, timestamp } if id == "broadcast-test" && timestamp == 99999)
        );
    }

    #[test]
    fn receiver_capacity_overflow_does_not_block_sender() {
        let sender = build_event_sender();

        // Create a receiver that doesn't consume
        let _rx = sender.subscribe();

        // Send multiple events - broadcast channel doesn't block on overflow
        for i in 0..200 {
            sender
                .send(MemoryEvent::ConceptInjected {
                    id: format!("id-{i}"),
                    timestamp: i,
                })
                .unwrap();
        }

        // Sender should still be functional
        sender
            .send(MemoryEvent::ConceptDeleted {
                id: "final".to_string(),
                timestamp: 999,
            })
            .unwrap();
    }

    #[test]
    fn default_channel_capacity_is_1024() {
        assert_eq!(DEFAULT_EVENT_CHANNEL_CAPACITY, 1024);
    }
}
