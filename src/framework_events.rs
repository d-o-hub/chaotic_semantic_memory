use tokio::sync::broadcast;

#[cfg(target_arch = "wasm32")]
use crate::error::Result;
use crate::framework::ChaoticSemanticFramework;
#[cfg(target_arch = "wasm32")]
use crate::hyperdim::HVec10240;
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
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub(crate) fn build_event_sender() -> broadcast::Sender<MemoryEvent> {
    broadcast::channel(DEFAULT_EVENT_CHANNEL_CAPACITY).0
}
