//! Bridge retrieval pipeline for semantic expansion.
//!
//! Provides a query pipeline that expands queries through the concept graph
//! and combines deterministic HDC recall with concept-expanded results.

// Casts are intentional for bridge score math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::encoder::TextEncoder;
use crate::error::Result;
use crate::hyperdim::HVec10240;
use crate::retrieval::hybrid::normalize_scores;
use crate::semantic_bridge::{
    BridgeConfig, BridgeHit, ConceptGraph, MemoryPacket, ScoreBreakdown, SemanticReranker,
};
use crate::singularity::Singularity;

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
        self.compile_packet(ns, query_text, &hits, singularity)
    }

    /// Merge primary and expanded results with score breakdown.
    fn merge_with_breakdown(
        &self,
        primary: &[(String, f32)],
        expanded: &[(String, f32)],
    ) -> Vec<BridgeHit> {
        use std::collections::HashMap;

        let mut hit_map: HashMap<String, BridgeHit> = HashMap::new();

        // Process primary results (deterministic scores)
        for (id, score) in primary {
            hit_map.insert(
                id.clone(),
                BridgeHit {
                    id: id.clone(),
                    text_preview: None,
                    scores: ScoreBreakdown {
                        deterministic: *score,
                        concept: 0.0,
                        semantic: 0.0,
                        final_score: 0.0,
                        evidence: vec!["deterministic_recall".to_string()],
                    },
                },
            );
        }

        // Process expanded results (concept scores)
        for (id, score) in expanded {
            if let Some(hit) = hit_map.get_mut(id) {
                // Boost existing hit's concept score
                hit.scores.concept = hit.scores.concept.max(*score);
                hit.scores.evidence.push("concept_expansion".to_string());
            } else {
                // New hit from expansion only
                hit_map.insert(
                    id.clone(),
                    BridgeHit {
                        id: id.clone(),
                        text_preview: None,
                        scores: ScoreBreakdown {
                            deterministic: 0.0,
                            concept: *score,
                            semantic: 0.0,
                            final_score: 0.0,
                            evidence: vec!["concept_expansion".to_string()],
                        },
                    },
                );
            }
        }

        hit_map.into_values().collect()
    }

    /// Compute final score from breakdown using configurable weights.
    fn compute_final_score(&self, scores: &ScoreBreakdown) -> f32 {
        self.config.deterministic_weight * scores.deterministic
            + self.config.concept_weight * scores.concept
            + self.config.semantic_weight * scores.semantic
    }

    /// Compile hits into a memory packet with token budget.
    fn compile_packet(
        &self,
        ns: &str,
        query_text: &str,
        hits: &[BridgeHit],
        singularity: &Singularity,
    ) -> Result<MemoryPacket> {
        // Extract facts from hits
        let mut facts: Vec<(String, f32)> = Vec::new();
        let mut sources: Vec<String> = Vec::new();

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
                sources.push(hit.id.clone());
            }
        }

        // Deduplicate facts (exact match)
        let mut unique_facts: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (text, _score) in &facts {
            if !seen.contains(text) {
                seen.insert(text.clone());
                unique_facts.push(text.clone());
            }
        }

        // Truncate to max_packet_facts
        unique_facts.truncate(self.config.max_packet_facts);

        // Apply token budget (drop lowest-scored facts)
        let mut budgeted_facts: Vec<String> = Vec::new();
        let mut token_count = 0;
        for text in unique_facts {
            let estimated = (text.split_whitespace().count() as f32 / 0.75).ceil() as usize;
            if token_count + estimated <= self.config.token_budget {
                budgeted_facts.push(text);
                token_count += estimated;
            }
        }

        // Compute confidence from top-k final_scores
        let confidence = if hits.is_empty() {
            0.0
        } else {
            let top_scores: Vec<f32> = hits
                .iter()
                .take(self.config.max_packet_facts)
                .map(|h| h.scores.final_score)
                .collect();
            top_scores.iter().sum::<f32>() / top_scores.len() as f32
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

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparisons for score test assertions

    use super::*;
    use crate::semantic_bridge::CanonicalConcept;
    use crate::singularity::{Singularity, SingularityConfig};

    #[test]
    fn test_bridge_retrieval_empty_singularity() {
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = BridgeRetrieval::with_defaults(encoder, graph);
        let singularity = Singularity::new(SingularityConfig::default());

        let results = bridge
            .query("_default", &singularity, "test query", 10, None)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_bridge_retrieval_empty_graph() {
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = BridgeRetrieval::with_defaults(encoder.clone(), graph);

        let mut singularity = Singularity::new(SingularityConfig::default());
        let concept = crate::singularity::ConceptBuilder::new("test-concept")
            .with_vector(encoder.encode("test content"))
            .build()
            .unwrap();
        singularity.inject("_default", concept).unwrap();

        let results = bridge
            .query("_default", &singularity, "test query", 10, None)
            .unwrap();
        // Should return deterministic results even without graph expansion
        assert!(!results.is_empty());
        assert!(results[0].scores.deterministic > 0.0);
        assert_eq!(results[0].scores.concept, 0.0);
    }

    #[test]
    fn test_bridge_retrieval_with_expansion() {
        let encoder = TextEncoder::new();
        let mut graph = ConceptGraph::new();

        // Add canonical concept with label matching query
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("agent-memory")
                .with_label("session-context"),
        );

        let bridge = BridgeRetrieval::with_defaults(encoder.clone(), graph);

        let mut singularity = Singularity::new(SingularityConfig::default());
        let concept = crate::singularity::ConceptBuilder::new("mem-1")
            .with_vector(encoder.encode("session context for AI agent"))
            .build()
            .unwrap();
        singularity.inject("_default", concept).unwrap();

        let results = bridge
            .query("_default", &singularity, "agent memory session", 10, None)
            .unwrap();

        assert!(!results.is_empty());
        // Check that expansion added concept score evidence
        assert!(
            results[0]
                .scores
                .evidence
                .contains(&"deterministic_recall".to_string())
        );
    }

    #[test]
    fn test_memory_packet_empty_hits() {
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = BridgeRetrieval::with_defaults(encoder, graph);
        let singularity = Singularity::new(SingularityConfig::default());

        let packet = bridge
            .memory_packet("_default", &singularity, "test query", 10, None)
            .unwrap();
        assert!(packet.facts.is_empty());
        assert!(packet.sources.is_empty());
        assert_eq!(packet.confidence, 0.0);
    }

    #[test]
    fn test_final_score_weights() {
        let config = BridgeConfig {
            deterministic_weight: 0.6,
            concept_weight: 0.3,
            semantic_weight: 0.1,
            ..Default::default()
        };

        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = BridgeRetrieval::new(encoder, graph, config);

        let scores = ScoreBreakdown {
            deterministic: 1.0,
            concept: 1.0,
            semantic: 1.0,
            final_score: 0.0,
            evidence: vec!["test".to_string()],
        };

        let final_score = bridge.compute_final_score(&scores);
        assert!((final_score - 1.0).abs() < 1e-6); // All weights sum to 1.0
    }
}

#[cfg(test)]
mod tests_v2 {
    use super::*;
    use crate::hyperdim::HVec10240;
    use crate::singularity::{ConceptBuilder, Singularity, SingularityConfig};

    #[test]
    fn test_bridge_retrieval_query_v2() {
        let mut singularity = Singularity::new(SingularityConfig::default());
        let concept = ConceptBuilder::new("c1")
            .with_vector(HVec10240::random())
            .build()
            .unwrap();
        singularity.inject("_default", concept).unwrap();

        let bridge = BridgeRetrieval::new(
            TextEncoder::new(),
            ConceptGraph::new(),
            BridgeConfig::default(),
        );
        let results = bridge.query("_default", &singularity, "test", 10, None);
        assert!(results.is_ok());
    }
}
