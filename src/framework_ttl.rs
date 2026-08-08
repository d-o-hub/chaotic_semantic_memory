//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::absence_ops::{known_absence_entry, short_circuit_abstention};
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
use crate::bridge_persistence::{AbsenceStore, persist_absence};
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
    /// If the query is known-absent at the configured threshold, return an
    /// immediate abstention without embedding or searching.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn short_circuit_if_known_absent(
        &self,
        query: &str,
    ) -> Option<HybridResult> {
        let min_attempts = self.config.absence_short_circuit_min_attempts;
        let store = self.persistence.as_ref()?;
        let ns = self.namespace().await;
        let entry = known_absence_entry(&ns, query, store.as_ref(), min_attempts).await?;
        Some(short_circuit_abstention(query, &entry))
    }

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "persistence")))]
    #[allow(clippy::unused_async)]
    pub(crate) async fn short_circuit_if_known_absent(
        &self,
        _query: &str,
    ) -> Option<HybridResult> {
        None
    }

    /// Clear absence rows after memory mutations so inject can resurrect queries.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn invalidate_absence_memory(&self) {
        if let Some(ref store) = self.persistence {
            if let Err(e) = store.clear_absences().await {
                tracing::warn!("Failed to clear absence entries after inject: {e}");
            }
        }
    }

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "persistence")))]
    #[allow(clippy::unused_async)]
    pub(crate) async fn invalidate_absence_memory(&self) {}

    /// Returns true when the query is known-absent at the configured threshold.
    ///
    /// Useful for CLI hybrid paths that should skip BM25 as well as HDC.
    pub async fn is_known_absent_query(&self, query: &str) -> bool {
        self.short_circuit_if_known_absent(query).await.is_some()
    }

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
        self.invalidate_absence_memory().await;

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

    /// Cancel the background TTL cleanup loop and wait (bounded) for it to exit.
    ///
    /// Idempotent: safe to call multiple times, and a no-op when cleanup was
    /// never started (`cleanup_interval_seconds == 0`). Waits up to 5 seconds
    /// for the task to observe cancellation; aborts it if the deadline
    /// elapses.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn shutdown_cleanup(&self) {
        self.ttl_cleanup_shutdown.cancel();
        let Some(handle) = self.ttl_cleanup_task.as_ref() else {
            return;
        };
        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while !handle.is_finished() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if !handle.is_finished() {
            handle.abort();
        }
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
        if let Some(short_circuit) = self.short_circuit_if_known_absent(query).await {
            return Ok(short_circuit);
        }

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
                let ns = self.namespace().await;
                if let Err(e) = persist_absence(&ns, &abstention, store.as_ref()).await {
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
        if let Some(short_circuit) = self.short_circuit_if_known_absent(query).await {
            return Ok(short_circuit);
        }

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
                let ns = self.namespace().await;
                if let Err(e) = persist_absence(&ns, &abstention, store.as_ref()).await {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    //! Owned TTL cleanup lifecycle: the background loop must never outlive
    //! (be orphaned by) the framework, and shutdown must be bounded.
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::ChaoticSemanticFramework;
    use crate::framework_ttl_advanced::TtlConfig;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    /// Wait up to `timeout` for the test-only flag proving the cleanup loop
    /// exited, then assert it.
    async fn assert_loop_exited(flag: &Arc<std::sync::atomic::AtomicBool>) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !flag.load(Ordering::SeqCst) && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(flag.load(Ordering::SeqCst), "cleanup task did not exit within 3s");
    }

    async fn build_with_interval(interval_secs: u64) -> ChaoticSemanticFramework {
        let mut ttl_config = TtlConfig::default();
        ttl_config.cleanup_interval_seconds = interval_secs;
        crate::ChaoticSemanticFramework::builder()
            .without_persistence()
            .with_ttl_config(ttl_config)
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn drop_inside_runtime_stops_cleanup_loop() {
        let framework = build_with_interval(1).await;
        let exited = framework.cleanup_loop_exited.clone();
        // Let the loop tick at least once before tearing down.
        tokio::time::sleep(Duration::from_millis(1200)).await;
        drop(framework);
        assert_loop_exited(&exited).await;
    }

    #[tokio::test]
    async fn drop_outside_runtime_stops_cleanup_loop() {
        let framework = build_with_interval(1).await;
        let exited = framework.cleanup_loop_exited.clone();
        tokio::time::sleep(Duration::from_millis(1200)).await;
        // Drop on a plain thread: the Drop impl must execute its bounded
        // block_on join path without panicking.
        std::thread::spawn(move || drop(framework))
            .join()
            .expect("drop thread panicked");
        assert_loop_exited(&exited).await;
    }

    #[tokio::test]
    async fn shutdown_cleanup_is_fast_and_idempotent_when_disabled() {
        let framework = build_with_interval(0).await;
        assert!(framework.ttl_cleanup_task.is_none());
        let started = Instant::now();
        framework.shutdown_cleanup().await;
        framework.shutdown_cleanup().await; // idempotent
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn shutdown_cleanup_stops_running_loop_within_deadline() {
        let framework = build_with_interval(1).await;
        let exited = framework.cleanup_loop_exited.clone();
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let started = Instant::now();
        framework.shutdown_cleanup().await;
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "shutdown_cleanup exceeded its 5s bound"
        );
        assert_loop_exited(&exited).await;
        // Owned lifecycle: the handle is still tracked (already finished).
        assert!(framework.ttl_cleanup_task.as_ref().is_some_and(|h| h.is_finished()));
    }
}
