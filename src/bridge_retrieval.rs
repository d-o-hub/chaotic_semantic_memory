//! Bridge retrieval pipeline for semantic expansion.
//!
//! Provides a query pipeline that expands queries through the concept graph
//! and combines deterministic HDC recall with concept-expanded results.

// Casts are intentional for bridge score math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::retrieval::hybrid::normalize_scores;
use crate::semantic_bridge::{
    BridgeConfig, BridgeHit, ConceptGraph, MemoryPacket, ScoreBreakdown, SemanticReranker,
};
use crate::singularity::Singularity;
use csm_core::encoder::TextEncoder;
use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;

/// Bridge retrieval orchestrator combining concept expansion with HDC recall.
#[derive(Debug, Clone)]
pub struct BridgeRetrieval {
    /// Text encoder for query normalization.
    encoder: TextEncoder,
    /// Concept graph for semantic expansion.
    concept_graph: ConceptGraph,
    /// Configuration for retrieval behavior.
    config: BridgeConfig,
}

impl BridgeRetrieval {
    /// Create a new bridge retrieval pipeline.
    pub const fn new(
        encoder: TextEncoder,
        concept_graph: ConceptGraph,
        config: BridgeConfig,
    ) -> Self {
        Self {
            encoder,
            concept_graph,
            config,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(encoder: TextEncoder, concept_graph: ConceptGraph) -> Self {
        Self::new(encoder, concept_graph, BridgeConfig::default())
    }

    /// Execute the full bridge retrieval pipeline.
    ///
    /// Pipeline steps:
    /// 1. Normalize and encode query
    /// 2. First recall: deterministic HDC similarity
    /// 3. Concept expansion via graph matching
    /// 4. Second recall: expanded query vector
    /// 5. Merge and score with breakdown
    /// 6. Optional reranking
    pub fn query(
        &self,
        ns: &str,
        singularity: &Singularity,
        query_text: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<Vec<BridgeHit>> {
        if top_k == 0 || singularity.is_empty(ns) {
            return Ok(Vec::new());
        }

        // Step 1: Normalize and encode primary query
        let tokens = TextEncoder::tokenize(query_text, self.encoder.config().code_aware, true);
        let query_hv = self.encoder.encode(query_text);

        // Step 2: First recall - deterministic HDC scores
        let primary_results = singularity.find_similar(ns, &query_hv, top_k);
        let primary_normalized = normalize_scores(&primary_results);

        // Step 3: Concept expansion
        let matched_ids = self.concept_graph.match_tokens(&tokens);
        let expanded_labels = self
            .concept_graph
            .expand(&matched_ids, self.config.max_expansion_depth);

        // Step 4: Encode expanded labels for second recall (if any matches)
        let expanded_results = if expanded_labels.is_empty() {
            Vec::new()
        } else {
            // Bundle expanded label vectors
            let label_hvs: Vec<HVec10240> = expanded_labels
                .iter()
                .map(|label| self.encoder.encode(label))
                .collect();

            let expanded_hv = HVec10240::bundle(&label_hvs).unwrap_or_else(|_| HVec10240::zero());
            let results = singularity.find_similar(ns, &expanded_hv, top_k);
            normalize_scores(&results)
        };

        // Step 5: Merge results with score breakdown
        let mut hits = self.merge_with_breakdown(&primary_normalized, &expanded_results);

        // Step 6: Optional reranking (never mutates deterministic scores)
        if let Some(reranker) = reranker {
            reranker.rerank(query_text, &mut hits);
        }

        // Compute final scores using configurable weights
        for hit in &mut hits {
            hit.scores.final_score = self.compute_final_score(&hit.scores);
        }

        // Sort by final score and truncate
        hits.sort_by(|a, b| b.scores.final_score.total_cmp(&a.scores.final_score));
        hits.truncate(top_k);

        Ok(hits)
    }

    /// Execute the bridge retrieval pipeline and return results with best score seen.
    pub fn query_with_best_score(
        &self,
        ns: &str,
        singularity: &Singularity,
        query_text: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<(Vec<BridgeHit>, f32)> {
        let hits = self.query(ns, singularity, query_text, top_k, reranker)?;
        let best_score = hits.first().map_or(0.0, |h| h.scores.final_score);
        Ok((hits, best_score))
    }

    /// Compile a memory packet from query results.
    ///
    /// Calls `query()` then compiles hits into a compressed packet
    /// suitable for LLM context injection.
    pub fn memory_packet(
        &self,
        ns: &str,
        singularity: &Singularity,
        query_text: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<MemoryPacket> {
        let hits = self.query(ns, singularity, query_text, top_k, reranker)?;
        // Optimization: Pass hits by value to allow ownership transfer of concept IDs.
        self.compile_packet(ns, query_text, hits, singularity)
    }

    /// Merge primary and expanded results with score breakdown.
    fn merge_with_breakdown(
        &self,
        primary: &[(String, f32)],
        expanded: &[(String, f32)],
    ) -> Vec<BridgeHit> {
        use std::collections::HashMap;
        use std::collections::hash_map::Entry;
        // Optimization: Pre-allocate map to avoid redundant re-hashes and re-allocs.
        // Use &str as key to avoid redundant String clones during accumulation.
        let mut hit_map: HashMap<&str, BridgeHit> =
            HashMap::with_capacity(primary.len() + expanded.len());
        // Process primary results (deterministic scores)
        for (id, score) in primary {
            hit_map.insert(
                id.as_str(),
                BridgeHit {
                    id: id.clone(),
                    text_preview: None,
                    scores: ScoreBreakdown {
                        deterministic: *score,
                        concept: 0.0,
                        semantic: 0.0,
                        final_score: 0.0,
                        // Optimization: Pre-allocate evidence vector.
                        evidence: {
                            let mut v = Vec::with_capacity(2);
                            v.push("deterministic_recall".to_string());
                            v
                        },
                    },
                },
            );
        }

        // Process expanded results (concept scores)
        for (id, score) in expanded {
            match hit_map.entry(id.as_str()) {
                Entry::Occupied(mut entry) => {
                    let hit = entry.get_mut();
                    // Boost existing hit's concept score
                    hit.scores.concept = hit.scores.concept.max(*score);
                    hit.scores.evidence.push("concept_expansion".to_string());
                }
                Entry::Vacant(entry) => {
                    // New hit from expansion only
                    entry.insert(BridgeHit {
                        id: id.clone(),
                        text_preview: None,
                        scores: ScoreBreakdown {
                            deterministic: 0.0,
                            concept: *score,
                            semantic: 0.0,
                            final_score: 0.0,
                            evidence: vec!["concept_expansion".to_string()],
                        },
                    });
                }
            }
        }

        hit_map.into_values().collect()
    }

    /// Compute final score from breakdown using configurable weights.
    pub fn compute_final_score(&self, scores: &ScoreBreakdown) -> f32 {
        self.config.deterministic_weight * scores.deterministic
            + self.config.concept_weight * scores.concept
            + self.config.semantic_weight * scores.semantic
    }

    /// Compile hits into a memory packet with token budget.
    fn compile_packet(
        &self,
        ns: &str,
        query_text: &str,
        hits: Vec<BridgeHit>,
        singularity: &Singularity,
    ) -> Result<MemoryPacket> {
        // Optimization: Pre-allocate vectors based on input size.
        let mut facts: Vec<(String, f32)> = Vec::with_capacity(hits.len());
        let mut sources: Vec<String> = Vec::with_capacity(hits.len());

        for hit in hits {
            // Get concept for text preview
            if let Some(concept) = singularity.get(ns, &hit.id) {
                // Extract text from metadata or use ID
                let text = concept
                    .metadata
                    .get("_text")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| hit.id.clone(), |s| s.to_string());

                facts.push((text, hit.scores.final_score));
                // Optimization: Transfer ownership of concept ID instead of cloning.
                sources.push(hit.id);
            }
        }

        // Deduplicate facts (exact match)
        let mut unique_facts: Vec<String> = Vec::with_capacity(facts.len());
        let mut seen: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(facts.len());
        for (text, _score) in &facts {
            if !seen.contains(text) {
                seen.insert(text.clone());
                unique_facts.push(text.clone());
            }
        }

        // Truncate to max_packet_facts
        unique_facts.truncate(self.config.max_packet_facts);

        // Apply token budget (drop lowest-scored facts)
        let mut budgeted_facts: Vec<String> = Vec::with_capacity(unique_facts.len());
        let mut token_count = 0;
        for text in unique_facts {
            let estimated = (text.split_whitespace().count() as f32 / 0.75).ceil() as usize;
            if token_count + estimated <= self.config.token_budget {
                budgeted_facts.push(text);
                token_count += estimated;
            }
        }

        // Compute confidence from top-k final_scores
        let confidence = if sources.is_empty() {
            0.0
        } else {
            // Optimization: Avoid intermediate Vec allocation for confidence calculation.
            let count = sources.len().min(self.config.max_packet_facts);
            let sum: f32 = facts.iter().take(count).map(|(_, s)| *s).sum();
            sum / count as f32
        };

        Ok(MemoryPacket {
            query_intent: query_text.to_string(),
            facts: budgeted_facts,
            sources,
            confidence,
        })
    }

    /// Get the underlying concept graph.
    pub const fn concept_graph(&self) -> &ConceptGraph {
        &self.concept_graph
    }

    /// Get the underlying encoder.
    pub const fn encoder(&self) -> &TextEncoder {
        &self.encoder
    }

    /// Get the configuration.
    pub const fn config(&self) -> &BridgeConfig {
        &self.config
    }
}
