use std::collections::HashMap;
use std::sync::Arc;

use crate::retrieval::hybrid::{HybridResult, RetrievalAbstention, compute_weights, merge_results};
use csm_core_lib::encoder::TextEncoder;
use csm_core_lib::error::{MemoryError, Result};
use csm_retrieval::HybridMode;

use crate::framework::ChaoticSemanticFramework;
use crate::metadata_filter::MetadataFilter;
use crate::retrieval::bm25::Bm25Index;
use crate::singularity::{Concept, unix_now_secs};

#[derive(Default)]
pub(crate) struct TextIndexCache {
    entries: HashMap<(String, bool), TextIndexEntry>,
    revisions: HashMap<String, u64>,
}

struct TextIndexEntry {
    revision: u64,
    index: Arc<Bm25Index>,
}

impl ChaoticSemanticFramework {
    /// Probe text with hybrid keyword and semantic retrieval.
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<HybridResult> {
        self.probe_text_with_mode(query, top_k, HybridMode::Auto)
            .await
    }

    /// Probe text with both retrieval paths constrained by metadata and expiry.
    pub async fn probe_text_filtered(
        &self,
        query: &str,
        top_k: usize,
        filter: &MetadataFilter,
    ) -> Result<HybridResult> {
        filter.validate()?;
        self.probe_text_with_filter(query, top_k, HybridMode::Auto, false, Some(filter))
            .await
    }

    /// Hybrid text retrieval shared by framework, CLI, MCP, and WASM callers.
    pub async fn probe_text_with_mode(
        &self,
        query: &str,
        top_k: usize,
        mode: HybridMode,
    ) -> Result<HybridResult> {
        self.probe_text_with_options(query, top_k, mode, false)
            .await
    }

    /// Hybrid text retrieval with an explicit tokenizer mode for source-code corpora.
    pub async fn probe_text_with_options(
        &self,
        query: &str,
        top_k: usize,
        mode: HybridMode,
        code_aware: bool,
    ) -> Result<HybridResult> {
        self.probe_text_with_filter(query, top_k, mode, code_aware, None)
            .await
    }

    pub(crate) async fn probe_text_with_filter(
        &self,
        query: &str,
        top_k: usize,
        mode: HybridMode,
        code_aware: bool,
        filter: Option<&MetadataFilter>,
    ) -> Result<HybridResult> {
        self.validate_top_k(top_k)?;
        if mode == HybridMode::SemanticOnly && filter.is_none() && !code_aware {
            return self.probe_text_semantic(query, top_k).await;
        }

        // The shared owner carries the absence lifecycle the per-path probes had
        // before consolidation: a query that abstained repeatedly against the
        // current content skips retrieval, and a hit clears the record. Only the
        // unfiltered probe participates — a record describes content, not a
        // filter, so a filtered miss must neither write one nor consume one that
        // would hide a match reachable under a different filter.
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let absence_lifecycle = filter.is_none();
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let scope_mode = if mode == HybridMode::SemanticOnly {
            // A semantic-only probe never reads the token index, so its record is
            // the same one `probe_text_semantic` writes regardless of tokenizer.
            crate::framework_ttl::ABSENCE_SCOPE_SEMANTIC_ONLY
        } else if code_aware {
            crate::framework_ttl::ABSENCE_SCOPE_HYBRID_CODE_AWARE
        } else {
            crate::framework_ttl::ABSENCE_SCOPE_HYBRID
        };
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let absence = if absence_lifecycle {
            Some(self.absence_short_circuit(query, scope_mode).await)
        } else {
            None
        };
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        if let Some(result) = absence.as_ref().and_then(|a| a.result.clone()) {
            return Ok(result);
        }

        let encoder = if code_aware {
            TextEncoder::new_code_aware()
        } else {
            TextEncoder::new()
        };
        let tokens = encoder.tokenize_search(query);
        let keyword_results = if mode == HybridMode::SemanticOnly {
            Vec::new()
        } else {
            self.keyword_results(&tokens, top_k, code_aware, filter)
                .await?
        };

        let (semantic_results, best_score) = if mode == HybridMode::KeywordOnly {
            (Vec::new(), None)
        } else {
            let embedding = self.embedding_provider.embed(query).await?;
            let vector = self
                .embedding_provider
                .project(&embedding, &self.projection);
            let results = if let Some(filter) = filter {
                self.probe_filtered(&vector, top_k, filter).await?
            } else {
                self.probe(vector, top_k).await?
            };
            let namespace = self.namespace().await;
            let best = self
                .singularity
                .read()
                .await
                .last_retrieval_stats(&namespace)
                .best_score_seen;
            (results, best)
        };

        let weights = match mode {
            HybridMode::Auto => compute_weights(tokens.len()),
            HybridMode::SemanticOnly => (0.0, 1.0),
            HybridMode::KeywordOnly => (1.0, 0.0),
            HybridMode::Custom(keyword_weight)
                if keyword_weight.is_finite() && (0.0..=1.0).contains(&keyword_weight) =>
            {
                (keyword_weight, 1.0 - keyword_weight)
            }
            HybridMode::Custom(_) => {
                return Err(MemoryError::InvalidInput {
                    field: "keyword_weight".to_string(),
                    reason: "keyword weight must be finite and between 0.0 and 1.0".to_string(),
                });
            }
        };
        let results = merge_results(&keyword_results, &semantic_results, weights, top_k);
        #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
        let (absence_revision, absence_record_id) =
            absence.map_or((0, None), |a| (a.revision, a.record_id));
        if results.is_empty() {
            let abstention = RetrievalAbstention {
                query: query.to_string(),
                min_score_threshold: self.config.pattern_recognition_threshold as f32,
                best_score_seen: best_score,
                attempted_modes: vec![match mode {
                    HybridMode::Auto => "Hybrid".to_string(),
                    HybridMode::SemanticOnly => "SemanticOnly".to_string(),
                    HybridMode::KeywordOnly => "KeywordOnly".to_string(),
                    HybridMode::Custom(_) => "CustomHybrid".to_string(),
                }],
                timestamp: chrono::Utc::now(),
            };

            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            if absence_lifecycle {
                self.persist_absence_record(&abstention, absence_revision, scope_mode)
                    .await;
            }

            Ok(HybridResult::Abstained(abstention))
        } else {
            #[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))]
            self.clear_absence_record(absence_record_id).await;

            Ok(HybridResult::Success(results))
        }
    }

    async fn keyword_results(
        &self,
        tokens: &[String],
        top_k: usize,
        code_aware: bool,
        filter: Option<&MetadataFilter>,
    ) -> Result<Vec<(String, f32)>> {
        let namespace = self.namespace().await;
        let index = self.text_index_for(&namespace, code_aware).await?;
        let sing = self.singularity.read().await;
        let now = unix_now_secs();
        let allowed = |id: &str| {
            sing.get(&namespace, id).is_some_and(|concept| {
                is_active(concept, now)
                    && filter.is_none_or(|filter| filter.matches(&concept.metadata))
            })
        };
        Ok(index.search_filtered(tokens, top_k, allowed))
    }

    pub(crate) async fn text_index_for(
        &self,
        namespace: &str,
        code_aware: bool,
    ) -> Result<Arc<Bm25Index>> {
        let key = (namespace.to_string(), code_aware);
        loop {
            let revision = {
                let cache = self.text_index_cache.read().await;
                let revision = cache.revisions.get(namespace).copied().unwrap_or(0);
                if let Some(entry) = cache.entries.get(&key)
                    && entry.revision == revision
                {
                    return Ok(Arc::clone(&entry.index));
                }
                drop(cache);
                revision
            };

            let encoder = if code_aware {
                TextEncoder::new_code_aware()
            } else {
                TextEncoder::new()
            };
            let mut index = Bm25Index::new();
            {
                let sing = self.singularity.read().await;
                for concept in sing.all_concepts(namespace) {
                    let Some(text) = searchable_text(&concept) else {
                        continue;
                    };
                    let tokens = encoder.tokenize_search(text);
                    if !tokens.is_empty() {
                        index.add_document(&concept.id, &tokens);
                    }
                }
            }
            let index = Arc::new(index);

            let mut cache = self.text_index_cache.write().await;
            let current_revision = cache.revisions.get(namespace).copied().unwrap_or(0);
            if revision != current_revision {
                continue;
            }
            if let Some(entry) = cache.entries.get(&key)
                && entry.revision == current_revision
            {
                return Ok(Arc::clone(&entry.index));
            }
            cache.entries.insert(
                key.clone(),
                TextIndexEntry {
                    revision,
                    index: Arc::clone(&index),
                },
            );
            drop(cache);
            return Ok(index);
        }
    }

    pub(crate) async fn invalidate_text_index(&self, namespace: &str) {
        let mut cache = self.text_index_cache.write().await;
        let revision = cache.revisions.entry(namespace.to_string()).or_default();
        *revision = revision.wrapping_add(1);
        cache
            .entries
            .retain(|(entry_namespace, _), _| entry_namespace != namespace);
    }
}

fn searchable_text(concept: &Concept) -> Option<&str> {
    concept
        .metadata
        .get("searchable_text")
        .or_else(|| concept.metadata.get("text_preview"))
        .or_else(|| concept.metadata.get("content_preview"))
        .and_then(serde_json::Value::as_str)
}

fn is_active(concept: &Concept, now: u64) -> bool {
    concept.expires_at.is_none_or(|expires_at| expires_at > now)
}
