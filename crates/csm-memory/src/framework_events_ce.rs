//! CloudEvents event emitter implementation (ADR-0078).

#[cfg(all(feature = "events-http", not(target_arch = "wasm32")))]
use csm_core::MemoryError;
#[cfg(any(feature = "cloudevents", feature = "events-http"))]
use csm_core::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[cfg(feature = "cloudevents")]
use crate::framework_events::MemoryEvent;
#[cfg(feature = "cloudevents")]
use cloudevents::{AttributesReader, Event, EventBuilder, EventBuilderV10};
#[cfg(feature = "cloudevents")]
use uuid::Uuid;

/// Storage target for binding events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageTarget {
    /// libSQL/SQLite persistence
    LibSql,
    /// In-memory only
    Memory,
}

/// Events emitted by the chaotic semantic memory framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChaoticEvent {
    /// Reservoir state converges to attractor basin.
    AttractorFired {
        /// Unique ID of the detected attractor.
        attractor_id: u32,
        /// Energy level of the basin (lower is more stable).
        basin_energy: f64,
        /// Dimension of the reservoir.
        reservoir_dim: usize,
    },
    /// HD vector similarity threshold exceeded.
    PatternRecognized {
        /// The query vector that triggered the match (binary serialized).
        query_vector: Vec<u8>,
        /// Key/ID of the matched concept.
        matched_key: String,
        /// Cosine similarity score.
        similarity: f64,
    },
    /// Consolidation cycle completes (e.g. TTL purge).
    MemoryConsolidated {
        /// Number of episodes/concepts processed.
        episode_count: usize,
        /// Duration of the cycle in milliseconds.
        duration_ms: u64,
    },
    /// Echo state network forward pass completes.
    EchoComputed {
        /// Dimension of the input vector.
        input_dim: usize,
        /// L2 norm of the new state.
        state_norm: f64,
    },
    /// New HD binding stored.
    BindingCreated {
        /// Key/ID of the binding.
        key: String,
        /// Dimension of the vector.
        dim: usize,
        /// Target storage backend.
        target: StorageTarget,
    },
}

#[cfg(feature = "cloudevents")]
impl ChaoticEvent {
    /// Map this event to a CloudEvent.
    pub fn to_cloud_event(&self, source: &str) -> Event {
        let (ce_type, data) = match self {
            Self::AttractorFired {
                attractor_id,
                basin_energy,
                reservoir_dim,
            } => (
                "io.d-o-hub.csm.attractor.fired",
                serde_json::json!({
                    "attractor_id": attractor_id,
                    "basin_energy": basin_energy,
                    "reservoir_dim": reservoir_dim,
                }),
            ),
            Self::PatternRecognized {
                query_vector,
                matched_key,
                similarity,
            } => (
                "io.d-o-hub.csm.pattern.recognized",
                serde_json::json!({
                    "query_vector": query_vector,
                    "matched_key": matched_key,
                    "similarity": similarity,
                }),
            ),
            Self::MemoryConsolidated {
                episode_count,
                duration_ms,
            } => (
                "io.d-o-hub.csm.memory.consolidated",
                serde_json::json!({
                    "episode_count": episode_count,
                    "duration_ms": duration_ms,
                }),
            ),
            Self::EchoComputed {
                input_dim,
                state_norm,
            } => (
                "io.d-o-hub.csm.echo.computed",
                serde_json::json!({
                    "input_dim": input_dim,
                    "state_norm": state_norm,
                }),
            ),
            Self::BindingCreated { key, dim, target } => (
                "io.d-o-hub.csm.binding.created",
                serde_json::json!({
                    "key": key,
                    "dim": dim,
                    "target": target,
                }),
            ),
        };

        EventBuilderV10::new()
            .id(Uuid::new_v4().to_string())
            .source(source)
            .ty(ce_type)
            .data("application/json", data)
            .build()
            .expect("valid CloudEvent")
    }
}

#[cfg(feature = "cloudevents")]
impl MemoryEvent {
    /// Map this memory event to a CloudEvent.
    pub fn to_cloud_event(&self, source: &str) -> Event {
        let (variant, subject, data) = match self {
            Self::ConceptInjected { id, .. } => (
                "injected",
                Some(id.clone()),
                serde_json::to_value(self).unwrap_or_default(),
            ),
            Self::ConceptUpdated { id, .. } => (
                "updated",
                Some(id.clone()),
                serde_json::to_value(self).unwrap_or_default(),
            ),
            Self::ConceptDeleted { id, .. } => (
                "deleted",
                Some(id.clone()),
                serde_json::to_value(self).unwrap_or_default(),
            ),
            Self::Associated { from, .. } => (
                "associated",
                Some(from.clone()),
                serde_json::to_value(self).unwrap_or_default(),
            ),
            Self::Disassociated { from, .. } => (
                "disassociated",
                Some(from.clone()),
                serde_json::to_value(self).unwrap_or_default(),
            ),
        };

        let ce_type = format!("io.d-o-hub.csm.memory.{}", variant);

        let mut builder = EventBuilderV10::new()
            .id(Uuid::new_v4().to_string())
            .source(source)
            .ty(ce_type)
            .data("application/json", data);

        if let Some(sub) = subject {
            builder = builder.subject(sub);
        }

        builder.build().expect("valid CloudEvent")
    }
}

/// Pluggable event emitter trait.
#[async_trait]
pub trait EventEmitter: Send + Sync + Debug {
    /// Name of the emitter.
    fn name(&self) -> &str;

    /// Emit a CloudEvent.
    #[cfg(feature = "cloudevents")]
    async fn emit(&self, event: Event) -> Result<()>;
}

/// Emitter that drops all events.
#[derive(Debug, Default)]
pub struct NullEmitter;

#[async_trait]
impl EventEmitter for NullEmitter {
    fn name(&self) -> &str {
        "null"
    }

    #[cfg(feature = "cloudevents")]
    async fn emit(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

/// Emitter that logs events using the tracing crate.
#[derive(Debug, Default)]
pub struct LogEmitter;

#[async_trait]
impl EventEmitter for LogEmitter {
    fn name(&self) -> &str {
        "log"
    }

    #[cfg(feature = "cloudevents")]
    async fn emit(&self, event: Event) -> Result<()> {
        tracing::info!(
            target: "chaotic_semantic_memory::events",
            ce_id = %event.id(),
            ce_type = %event.ty(),
            ce_source = %event.source(),
            "CloudEvent emitted"
        );
        Ok(())
    }
}

/// Emitter that sends events via HTTP POST.
#[cfg(all(feature = "events-http", not(target_arch = "wasm32")))]
#[derive(Debug)]
pub struct HttpEmitter {
    client: reqwest::Client,
    url: String,
}

#[cfg(all(feature = "events-http", not(target_arch = "wasm32")))]
impl HttpEmitter {
    /// Create a new HttpEmitter.
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
        }
    }
}

#[cfg(all(feature = "events-http", not(target_arch = "wasm32")))]
#[async_trait]
impl EventEmitter for HttpEmitter {
    fn name(&self) -> &str {
        "http"
    }

    async fn emit(&self, event: Event) -> Result<()> {
        let resp = self
            .client
            .post(&self.url)
            .json(&event)
            .send()
            .await
            .map_err(|e| MemoryError::External(format!("HTTP emission failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(MemoryError::External(format!(
                "HTTP emission returned status: {}",
                resp.status()
            )));
        }
        Ok(())
    }
}
