//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::bridge_persistence::persist_absence;
use crate::framework_events::MemoryEvent;
use crate::framework_ttl_advanced::TtlPolicy;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::hybrid::{HybridResult, RetrievalAbstention};
use crate::singularity::ConceptBuilder;
use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use std::collections::HashMap;
use tracing::instrument;

impl crate::framework::ChaoticSemanticFramework {
    /// Evaluate the TTL policy for a concept.
    pub(crate) async fn evaluate_ttl_policy(
        &self,
        id: &str,
        metadata: &HashMap<String, serde_json::Value>,
    ) -> Option<u64> {
        let policy = &self.config.ttl_config.policy;
        match policy {
            TtlPolicy::None => None,
            TtlPolicy::Fixed(ttl) => Some(*ttl),
            TtlPolicy::MetadataRule(rules) => {
                for rule in rules {
                    if let Some(val) = metadata.get(&rule.key) {
                        if val == &rule.value {
                            return Some(rule.ttl_seconds);
                        }
                    }
                }
                None
            }
            TtlPolicy::Inherit => {
                // Check outgoing associations (inheritance from what we point TO)
                let outgoing = self.get_associations(id).await.ok().unwrap_or_default();
                if let Some((source_id, _)) = outgoing.first() {
                    if let Ok(Some(concept)) = self.get_concept(source_id).await {
                        if let Some(exp) = concept.expires_at {
                            let now = crate::singularity::unix_now_secs();
                            if exp > now {
                                return Some(exp - now);
                            }
                        }
                    }
                }
                None
            }
        }
    }

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

        #[cfg(not(target_arch = "wasm32"))]
        let p_start = std::time::Instant::now();
        #[cfg(target_arch = "wasm32")]
        let p_start = Date::now();

        self.durable_inject_concept(concept.clone()).await?;

        if self.persistence.is_some() {
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

        let cascading = self.config.ttl_config.cascading_purge;

        let count = {
            let mut sing = self.singularity.write().await;
            let ns = self.namespace.read().await;
            sing.purge_expired_cascading(&ns, cascading)
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
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<HybridResult> {
        let embedding = self.embedding_provider.embed(query).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        let (results, best_score) = self.probe_with_best_score(vector, top_k).await?;

        if results.is_empty() {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: self.config.pattern_recognition_threshold as f32,
                best_score_seen: best_score,
                attempted_modes: vec!["Auto".to_string()],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if let Some(ref store) = self.persistence {
                if let Err(e) = persist_absence(&abstention, store.as_ref()).await {
                    tracing::warn!("Failed to persist absence entry: {e}");
                }
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            Ok(HybridResult::Success(results))
        }
    }

    /// Query for similar concepts and return best score seen.
    pub async fn probe_with_best_score(
        &self,
        query: HVec10240,
        top_k: usize,
    ) -> Result<(Vec<(String, f32)>, Option<f32>)> {
        let results = self.probe(query, top_k).await?;
        let ns = self.namespace.read().await;
        let best_score = self
            .singularity
            .read()
            .await
            .last_retrieval_stats(&ns)
            .best_score_seen;
        Ok((results, best_score))
    }

    /// Probe for similar concepts using text input and metadata filtering.
    pub async fn probe_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Result<HybridResult> {
        let embedding = self.embedding_provider.embed(query).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        let results = self.probe_filtered(&vector, top_k, filter).await?;

        if results.is_empty() {
            let ns = self.namespace.read().await;
            let best_score = self
                .singularity
                .read()
                .await
                .last_retrieval_stats(&ns)
                .best_score_seen;

            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: self.config.pattern_recognition_threshold as f32,
                best_score_seen: best_score,
                attempted_modes: vec!["Filtered".to_string()],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if let Some(ref store) = self.persistence {
                if let Err(e) = persist_absence(&abstention, store.as_ref()).await {
                    tracing::warn!("Failed to persist absence entry: {e}");
                }
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            Ok(HybridResult::Success(results))
        }
    }

    /// Query for a session using text input. Filters results to those with matching `session_id` metadata.
    pub async fn query_in_session(
        &self,
        query: &str,
        session_id: &str,
        top_k: usize,
    ) -> Result<HybridResult> {
        let filter = MetadataFilter::eq("session_id", session_id);
        self.probe_text_filtered(query, top_k, &filter).await
    }
}
