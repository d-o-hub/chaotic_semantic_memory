//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

use crate::error::Result;
use crate::framework_events::MemoryEvent;
use crate::hyperdim::HVec10240;
use crate::singularity::ConceptBuilder;
use std::collections::HashMap;
use tracing::instrument;

impl crate::framework::ChaoticSemanticFramework {
    /// Inject a concept with TTL (time to live) into memory.
    ///
    /// The concept will expire after `ttl_seconds` from creation.
    /// Expired concepts are automatically filtered during probe operations.
    #[instrument(err, skip(self, id, vector))]
    pub async fn inject_concept_with_ttl(
        &self,
        id: impl Into<String>,
        vector: HVec10240,
        ttl_seconds: u64,
    ) -> Result<()> {
        let id = id.into();
        Self::validate_concept_id(&id)?;
        let concept = ConceptBuilder::new(id.clone())
            .with_vector(vector)
            .with_ttl(ttl_seconds)
            .build()?;

        {
            let mut sing = self.singularity.write().await;
            sing.inject(concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            persistence.save_concept(&concept).await?;
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id,
            timestamp: concept.modified_at,
        });

        Ok(())
    }

    /// Inject a concept from text with TTL.
    #[instrument(err, skip(self, text))]
    pub async fn inject_text_with_ttl(&self, id: &str, text: &str, ttl_seconds: u64) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept_with_ttl(id, vector, ttl_seconds).await
    }

    /// Purge all expired concepts from memory.
    ///
    /// Returns the number of concepts removed.
    #[instrument(err, skip(self))]
    pub async fn purge_expired(&self) -> Result<usize> {
        let mut sing = self.singularity.write().await;
        let count = sing.purge_expired();
        Ok(count)
    }

    /// Inject a concept from text using the built-in encoder.
    ///
    /// The text is encoded to a hypervector using `TextEncoder` and stored
    /// with the given ID. This is a convenience method for the common case
    /// of storing text-based concepts.
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept(id, vector).await
    }

    /// Inject a concept from text with metadata.
    pub async fn inject_text_with_metadata(
        &self,
        id: &str,
        text: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(text);
        self.inject_concept_with_metadata(id, vector, metadata)
            .await
    }

    /// Probe for similar concepts using text input.
    ///
    /// Encodes the query text and finds the most similar concepts.
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let encoder = crate::encoder::TextEncoder::new();
        let vector = encoder.encode(query);
        self.probe(vector, top_k).await
    }
}
