//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

use csm_core::error::Result;
use crate::framework_events::MemoryEvent;
use csm_core::hyperdim::HVec10240;
use crate::metadata_filter::MetadataFilter;
use crate::singularity::ConceptBuilder;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use std::collections::HashMap;
use tracing::instrument;

impl crate::framework::ChaoticSemanticFramework {
    /// Inject a concept with TTL. The concept expires after `ttl_seconds`; expired concepts are filtered during probe.
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
            let ns = self.namespace.read().await;
            sing.inject(&ns, concept.clone())?;
        }

        if let Some(ref persistence) = self.persistence {
            #[cfg(not(target_arch = "wasm32"))]
            let p_start = std::time::Instant::now();
            #[cfg(target_arch = "wasm32")]
            let p_start = Date::now();

            let ns = self.namespace.read().await;
            persistence.save_concept(&ns, &concept).await?;

            #[cfg(not(target_arch = "wasm32"))]
            let elapsed_ms = u64::try_from(p_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            #[cfg(target_arch = "wasm32")]
            let elapsed_ms = (Date::now() - p_start) as u64;

            self.metrics.observe_persist_latency_ms(elapsed_ms, "save");
        }
        self.metrics.inc_concepts_injected(1);
        self.emit_event(MemoryEvent::ConceptInjected {
            id,
            timestamp: concept.modified_at,
        })
        .await;

        Ok(())
    }

    /// Inject a concept from text with TTL.
    #[instrument(err, skip(self, text))]
    pub async fn inject_text_with_ttl(&self, id: &str, text: &str, ttl_seconds: u64) -> Result<()> {
        let embedding = self.embedding_provider.embed(text).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.inject_concept_with_ttl(id, vector, ttl_seconds).await
    }

    /// Purge all expired concepts. Returns the count of concepts removed.
    #[instrument(err, skip(self))]
    pub async fn purge_expired(&self) -> Result<usize> {
        #[cfg(not(target_arch = "wasm32"))]
        let start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let start = Date::now();

        let count = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.purge_expired(&ns)
        };

        if count > 0 {
            #[cfg(not(target_arch = "wasm32"))]
            let duration_ms = start.elapsed().as_millis() as u64;
            #[cfg(target_arch = "wasm32")]
            let duration_ms = (Date::now() - start) as u64;

            self.emit_chaotic_event(
                crate::framework_events_ce::ChaoticEvent::MemoryConsolidated {
                    episode_count: count,
                    duration_ms,
                },
            )
            .await;
        }

        Ok(count)
    }

    /// Inject a concept from text using the embedding provider. Convenience for storing text-based concepts.
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()> {
        let embedding = self.embedding_provider.embed(text).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.inject_concept(id, vector).await
    }

    /// Inject a concept from text with metadata.
    pub async fn inject_text_with_metadata(
        &self,
        id: &str,
        text: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let embedding = self.embedding_provider.embed(text).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.inject_concept_with_metadata(id, vector, metadata)
            .await
    }

    /// Probe for similar concepts using text input. Encodes the query text via the embedding provider.
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>> {
        let embedding = self.embedding_provider.embed(query).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.probe(vector, top_k).await
    }

    /// Probe for similar concepts using text input and metadata filtering.
    pub async fn probe_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Result<Vec<(String, f32)>> {
        let embedding = self.embedding_provider.embed(query).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        self.probe_filtered(&vector, top_k, filter).await
    }

    /// Query for a session using text input. Filters results to those with matching `session_id` metadata.
    pub async fn query_in_session(
        &self,
        query: &str,
        session_id: &str,
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        let filter = MetadataFilter::eq("session_id", session_id);
        self.probe_text_filtered(query, top_k, &filter).await
    }
}
