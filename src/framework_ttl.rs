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

/// Retrieval-scope label of the hybrid text probe (`probe_text*`).
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_HYBRID: &str = "hybrid";
/// Retrieval-scope label of the code-aware tokenizer variant, whose keyword
/// misses say nothing about a default-tokenized probe.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_HYBRID_CODE_AWARE: &str = "hybrid-code-aware";
/// Retrieval-scope label of the semantic-only text probe.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_SEMANTIC_ONLY: &str = "semantic-only";
/// Retrieval-scope label of the unfiltered bridge probe.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_BRIDGE: &str = "bridge";
/// Retrieval-scope label of the reranked bridge probe.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_BRIDGE_RERANK: &str = "bridge-rerank";
/// Retrieval-scope label of the metadata-filtered bridge probe.
#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
pub(crate) const ABSENCE_SCOPE_BRIDGE_FILTERED: &str = "bridge-filtered";

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
                    if let Some(val) = metadata.get(&rule.key)
                        && val == &rule.value
                    {
                        return Some(rule.ttl_seconds);
                    }
                }
                None
            }
            TtlPolicy::Inherit => {
                // Check outgoing associations (inheritance from what we point TO)
                let outgoing = self.get_associations(id).await.ok().unwrap_or_default();
                if let Some((source_id, _)) = outgoing.first()
                    && let Ok(Some(concept)) = self.get_concept(source_id).await
                    && let Some(exp) = concept.expires_at
                {
                    let now = crate::singularity::unix_now_secs();
                    if exp > now {
                        return Some(exp - now);
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
        self.inject_concept_with_ttl_metadata(id.into(), vector, ttl_seconds, HashMap::new())
            .await
    }

    async fn inject_concept_with_ttl_metadata(
        &self,
        id: String,
        vector: HVec10240,
        ttl_seconds: u64,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        Self::validate_concept_id(&id)?;
        Self::validate_metadata_bytes(&metadata, self.config.max_metadata_bytes)?;
        let mut builder = ConceptBuilder::new(id.clone())
            .with_vector(vector)
            .with_ttl(ttl_seconds);
        for (key, value) in metadata {
            builder = builder.with_metadata(key, value);
        }
        let concept = builder.build()?;

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
        let metadata = HashMap::from([(
            "searchable_text".to_string(),
            serde_json::Value::String(text.to_string()),
        )]);
        self.inject_concept_with_ttl_metadata(id.to_string(), vector, ttl_seconds, metadata)
            .await
    }

    /// Inject a concept from text using the embedding provider. Convenience for storing text-based concepts.
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()> {
        self.inject_text_with_metadata(id, text, HashMap::new())
            .await
    }

    /// Inject a concept from text with metadata and preserve the searchable source text.
    pub async fn inject_text_with_metadata(
        &self,
        id: &str,
        text: &str,
        mut metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let embedding = self.embedding_provider.embed(text).await?;
        let vector = self
            .embedding_provider
            .project(&embedding, &self.projection);
        metadata.insert(
            "searchable_text".to_string(),
            serde_json::Value::String(text.to_string()),
        );
        self.inject_concept_with_metadata(id, vector, metadata)
            .await
    }

    /// M1: queries that abstained `ABSENCE_MIN_ATTEMPTS`+ times *against the
    /// current content* skip retrieval and abstain immediately.
    ///
    /// `scope_mode` names the probe family: an abstention only says that *that*
    /// retrieval path missed, so it must never suppress a different path that
    /// searches more (a semantic-only miss cannot hide a keyword match).
    ///
    /// Also reports the revision this check observed and the record id (when one
    /// exists for this namespace), so the caller can stamp a new abstention with
    /// the revision it actually observed and invalidate the record on success.
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    pub(crate) async fn absence_short_circuit(
        &self,
        query: &str,
        scope_mode: &str,
    ) -> AbsenceShortCircuit {
        let Some(store) = self.persistence.as_ref() else {
            return AbsenceShortCircuit {
                revision: 0,
                record_id: None,
                result: None,
            };
        };
        let namespace = self.namespace.read().await.clone();
        let Some(scope) = self.absence_scope_key(&namespace, scope_mode) else {
            return AbsenceShortCircuit {
                revision: 0,
                record_id: None,
                result: None,
            };
        };
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
        let lookup = crate::retrieval::bm25::lookup_absence_scoped(
            query,
            &scope,
            store.as_ref(),
            ABSENCE_MIN_ATTEMPTS,
            &namespace,
            revision,
        )
        .await;
        if lookup.short_circuit {
            #[cfg(feature = "prometheus")]
            crate::observability::prom::record_absence_short_circuit();
        }
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
    #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
    fn absence_scope_key(&self, namespace: &str, scope_mode: &str) -> Option<String> {
        if self.embedding_provider.name() != "hdc-text" {
            return None;
        }
        Some(format!(
            "text-absence-v1|namespace={namespace}|mode={scope_mode}|provider={}|dimension={}|threshold={:08x}",
            self.embedding_provider.name(),
            self.embedding_provider.native_dim(),
            self.config.pattern_recognition_threshold.to_bits(),
        ))
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
        scope_mode: &str,
    ) {
        let Some(store) = self.persistence.as_ref() else {
            return;
        };
        let namespace = self.namespace.read().await.clone();
        let Some(scope) = self.absence_scope_key(&namespace, scope_mode) else {
            return;
        };
        if let Err(e) = crate::bridge_persistence::persist_absence_scoped(
            abstention,
            store.as_ref(),
            &namespace,
            namespace_revision,
            &scope,
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

    /// Probe using semantic-only HDC retrieval and the configured embedding provider.
    pub async fn probe_text_semantic(&self, query: &str, top_k: usize) -> Result<HybridResult> {
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let absence = self
            .absence_short_circuit(query, ABSENCE_SCOPE_SEMANTIC_ONLY)
            .await;
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
            self.persist_absence_record(&abstention, absence.revision, ABSENCE_SCOPE_SEMANTIC_ONLY)
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
}
