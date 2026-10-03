//! TTL (Time-To-Live) and text convenience operations for ChaoticSemanticFramework.

use crate::concept_builder::ConceptBuilder;
use crate::framework_events::MemoryEvent;
use crate::framework_ttl_advanced::TtlPolicy;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::hybrid::{HybridResult, RetrievalAbstention};
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::HVec10240;
#[cfg(target_arch = "wasm32")]
use js_sys::Date;
use std::collections::HashMap;
use tracing::instrument;

/// Minimum persisted absence attempts before a query short-circuits retrieval (M1).
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub const ABSENCE_MIN_ATTEMPTS: u32 = 3;

/// Outcome of the persisted-absence check for one query.
///
/// Deliberately has no inherent methods: the fields are read directly by the
/// probe paths, which keeps the generated public API documents free of an
/// `impl` block for a crate-private type.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) struct AbsenceShortCircuit {
    /// Namespace revision this check observed. Records written by the same probe
    /// are stamped with it, so an abstention is attributed to the content state
    /// the probe saw — not to whatever a concurrent writer produced while the
    /// retrieval ran.
    pub revision: u64,
    /// Record id when one exists for this namespace (invalidation target once
    /// the query retrieves successfully).
    pub record_id: Option<String>,
    /// Short-circuit result when the record is authoritative for this content.
    pub result: Option<crate::retrieval::hybrid::HybridResult>,
}

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

    /// M1: queries that abstained `ABSENCE_MIN_ATTEMPTS`+ times *against the
    /// current content* skip retrieval and abstain immediately.
    ///
    /// Also reports the revision this check observed and the record id (when one
    /// exists for this namespace), so the caller can stamp a new abstention with
    /// the revision it actually observed and invalidate the record on success.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn absence_short_circuit(&self, query: &str) -> AbsenceShortCircuit {
        let Some(store) = self.persistence.as_ref() else {
            return AbsenceShortCircuit {
                revision: 0,
                record_id: None,
                result: None,
            };
        };
        let namespace = self.namespace.read().await.clone();
        // A record is only authoritative for the content it was observed
        // against, so the decision needs the namespace's current revision —
        // bumped by every durable concept mutation (ADR-0093). An unreadable
        // revision must not fall back to a default: skipping the short-circuit
        // is safe, suppressing a query on a guessed revision is not.
        let Some(revision) = self.namespace_revision().await else {
            tracing::warn!("Failed to read namespace revision; skipping absence short-circuit");
            return AbsenceShortCircuit {
                revision: 0,
                record_id: None,
                result: None,
            };
        };
        let lookup = crate::retrieval::bm25::lookup_absence(
            query,
            store.as_ref(),
            ABSENCE_MIN_ATTEMPTS,
            &namespace,
            revision,
        )
        .await;
        let result = lookup.short_circuit.then(|| {
            crate::retrieval::hybrid::HybridResult::Abstained(
                crate::retrieval::hybrid::RetrievalAbstention {
                    query: query.to_string(),
                    min_score_threshold: self.config.pattern_recognition_threshold as f32,
                    best_score_seen: None,
                    attempted_modes: vec!["AbsenceShortCircuit".to_string()],
                    timestamp: chrono::Utc::now(),
                },
            )
        });
        AbsenceShortCircuit {
            revision,
            record_id: lookup.entry_id,
            result,
        }
    }

    /// Current namespace revision, or `None` when persistence cannot read it.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn namespace_revision(&self) -> Option<u64> {
        let store = self.persistence.as_ref()?;
        let namespace = self.namespace.read().await.clone();
        store.get_namespace_revision(&namespace).await.ok()
    }

    /// Persist an abstention stamped with the current namespace and the given
    /// content revision.
    ///
    /// The caller passes the revision it observed *before* retrieval: an
    /// abstention describes the content state the probe actually saw, not the
    /// state a concurrent writer may have produced while it ran.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn persist_absence_record(
        &self,
        abstention: &RetrievalAbstention,
        namespace_revision: u64,
    ) {
        let Some(store) = self.persistence.as_ref() else {
            return;
        };
        let namespace = self.namespace.read().await.clone();
        if let Err(e) = crate::bridge_persistence::persist_absence(
            abstention,
            store.as_ref(),
            &namespace,
            namespace_revision,
        )
        .await
        {
            tracing::warn!("Failed to persist absence entry: {e}");
        }
    }

    /// Drop the absence record of a query that has just retrieved successfully.
    ///
    /// The record claimed the query matches nothing; the successful retrieval
    /// disproves it, so keeping it would suppress a query the store can answer.
    /// Does nothing when no record exists for this namespace, which keeps the
    /// successful-probe path free of extra store writes.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn clear_absence_record(&self, record_id: Option<String>) {
        let (Some(id), Some(store)) = (record_id, self.persistence.as_ref()) else {
            return;
        };
        if let Err(e) = csm_traits::AbsenceStore::delete_absence(store.as_ref(), &id).await {
            tracing::warn!("Failed to clear absence record: {e}");
        }
    }

    /// Probe for similar concepts using text input. Encodes the query text via the embedding provider.
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<HybridResult> {
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let absence = self.absence_short_circuit(query).await;
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        if let Some(result) = absence.result {
            return Ok(result);
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
            self.persist_absence_record(&abstention, absence.revision)
                .await;

            Ok(HybridResult::Abstained(abstention))
        } else {
            // The query is answerable now: drop the record that claimed
            // otherwise instead of leaving it to suppress a later probe.
            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            self.clear_absence_record(absence.record_id).await;

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
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let absence = self.absence_short_circuit(query).await;
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        if let Some(result) = absence.result {
            return Ok(result);
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
            self.persist_absence_record(&abstention, absence.revision)
                .await;

            Ok(HybridResult::Abstained(abstention))
        } else {
            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            self.clear_absence_record(absence.record_id).await;

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

#[cfg(all(test, not(target_arch = "wasm32"), feature = "persistence"))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::framework_builder::FrameworkBuilder;

    /// `namespace_revision` must report the live revision: a namespace with no
    /// durable writes reads 0 and a durable insert advances it. Revision-stamped
    /// state (absence records) depends on this being a real read, not a default.
    #[tokio::test]
    async fn namespace_revision_tracks_durable_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("revision.db");
        let fw = FrameworkBuilder::new()
            .with_local_db(db.to_str().unwrap())
            .build()
            .await
            .unwrap();

        assert_eq!(
            fw.namespace_revision().await,
            Some(0),
            "a namespace with no durable writes reads revision 0"
        );

        fw.inject_concept("c1", HVec10240::random()).await.unwrap();

        assert_eq!(
            fw.namespace_revision().await,
            Some(1),
            "a durable insert must advance the namespace revision"
        );
    }

    /// Both halves of the absence contract, observable through the store: an
    /// abstaining probe records the absence against the revision it observed,
    /// and a successful retrieval afterwards clears that record.
    #[tokio::test]
    async fn abstentions_are_recorded_and_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("absence.db");
        let fw = FrameworkBuilder::new()
            .with_local_db(db.to_str().unwrap())
            .build()
            .await
            .unwrap();
        let store = fw.persistence.as_ref().expect("persistence enabled");
        let query = "content-that-does-not-exist-yet";
        let id = csm_traits::AbsenceEntry::id_for(query);

        assert!(
            matches!(
                fw.probe_text(query, 5).await.unwrap(),
                HybridResult::Abstained(_)
            ),
            "an empty store must abstain"
        );
        let recorded = csm_traits::AbsenceStore::get_absence(store.as_ref(), &id)
            .await
            .unwrap()
            .expect("an abstention must persist an absence record");
        assert_eq!(
            recorded.namespace,
            fw.namespace().await,
            "the record must be scoped to the namespace it was observed in"
        );
        assert_eq!(
            recorded.namespace_revision, 0,
            "the record must carry the revision the probe observed"
        );
        assert_eq!(recorded.attempt_count, 1);

        // Content the query matches: the next probe retrieves successfully and
        // must clear the record instead of leaving it to suppress the query.
        fw.inject_text("answer", query).await.unwrap();
        assert!(
            matches!(
                fw.probe_text(query, 5).await.unwrap(),
                HybridResult::Success(_)
            ),
            "a matching concept must be retrievable"
        );
        assert!(
            csm_traits::AbsenceStore::get_absence(store.as_ref(), &id)
                .await
                .unwrap()
                .is_none(),
            "a successful retrieval must clear the absence record"
        );
    }
}
