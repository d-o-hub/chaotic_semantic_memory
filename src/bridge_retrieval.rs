//! Semantic Bridge retrieval implementation (ADR-0044).

use tracing::instrument;

use crate::encoder::TextEncoder;
use crate::error::Result;
use crate::semantic_bridge::{
    BridgeConfig, BridgeHit, ConceptGraph, MemoryPacket, ScoreBreakdown, SemanticReranker,
};
use crate::singularity::Singularity;

/// High-level retrieval pipeline that bridges text queries to conceptual memory.
pub struct BridgeRetrieval {
    encoder: TextEncoder,
    concept_graph: ConceptGraph,
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
        Self {
            encoder,
            concept_graph,
            config: BridgeConfig::default(),
        }
    }

    /// Query conceptual memory using a natural language string.
    #[instrument(skip(self, singularity, query_text, reranker), fields(query = %query_text))]
    pub fn query(
        &self,
        ns: &str,
        singularity: &Singularity,
        query_text: &str,
        top_k: usize,
        reranker: Option<&dyn SemanticReranker>,
    ) -> Result<Vec<BridgeHit>> {
        // Step 1: Encode query to hypervector
        let query_hv = self.encoder.encode(query_text);

        if top_k == 0 || singularity.is_empty(ns) {
            return Ok(Vec::new());
        }

        // Step 2: Deterministic recall from Singularity
        let primary_results = singularity.find_similar(ns, &query_hv, top_k);

        // Step 3: Semantic/Concept expansion via ConceptGraph
        let tokens: Vec<String> = query_text.split_whitespace().map(|s| s.to_string()).collect();
        let expanded_ids = self.concept_graph.match_tokens(&tokens);

        let mut expanded_results = Vec::new();
        for id in expanded_ids {
            if let Some(c) = singularity.get(ns, &id) {
                let sim = query_hv.cosine_similarity(&c.vector);
                expanded_results.push((id.clone(), sim));
            }
        }

        // Step 4: Merge results
        let mut hits = self.merge_with_breakdown(&primary_results, &expanded_results);

        // Step 5: Optional reranking
        if let Some(reranker) = reranker {
            reranker.rerank(query_text, &mut hits);
        }

        // Step 6: Final scoring
        for hit in &mut hits {
            hit.scores.final_score = self.compute_final_score(&hit.scores);
        }

        hits.sort_by(|a, b| b.scores.final_score.total_cmp(&a.scores.final_score));
        hits.truncate(top_k);

        Ok(hits)
    }

    /// Compile a memory packet from query results.
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

    fn merge_with_breakdown(
        &self,
        primary: &[(String, f32)],
        expanded: &[(String, f32)],
    ) -> Vec<BridgeHit> {
        use std::collections::HashMap;
        let mut hit_map: HashMap<String, BridgeHit> = HashMap::new();

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

        for (id, score) in expanded {
            if let Some(hit) = hit_map.get_mut(id) {
                hit.scores.concept = hit.scores.concept.max(*score);
                hit.scores.evidence.push("concept_expansion".to_string());
            } else {
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

    fn compute_final_score(&self, scores: &ScoreBreakdown) -> f32 {
        self.config.deterministic_weight * scores.deterministic
            + self.config.concept_weight * scores.concept
            + self.config.semantic_weight * scores.semantic
    }

    fn compile_packet(
        &self,
        ns: &str,
        query_text: &str,
        hits: &[BridgeHit],
        singularity: &Singularity,
    ) -> Result<MemoryPacket> {
        let mut facts: Vec<String> = Vec::new();
        let mut sources: Vec<String> = Vec::new();

        for hit in hits {
            if let Some(concept) = singularity.get(ns, &hit.id) {
                let text = concept
                    .metadata
                    .get("_text")
                    .and_then(|v| v.as_str())
                    .map_or_else(|| hit.id.clone(), |s| s.to_string());

                facts.push(text);
                sources.push(hit.id.clone());
            }
        }

        facts.truncate(self.config.max_packet_facts);

        let confidence = if hits.is_empty() {
            0.0
        } else {
            let top_scores: Vec<f32> = hits
                .iter()
                .take(self.config.max_packet_facts)
                .map(|h| h.scores.final_score)
                .collect();
            #[allow(clippy::cast_precision_loss)]
            let count = top_scores.len() as f32;
            top_scores.iter().sum::<f32>() / count
        };

        Ok(MemoryPacket {
            query_intent: query_text.to_string(),
            facts,
            sources,
            confidence,
        })
    }

    pub const fn concept_graph(&self) -> &ConceptGraph {
        &self.concept_graph
    }

    pub const fn encoder(&self) -> &TextEncoder {
        &self.encoder
    }

    pub const fn config(&self) -> &BridgeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singularity::{Singularity, SingularityConfig};

    #[test]
    fn test_bridge_retrieval_empty_singularity() {
        let encoder = TextEncoder::new();
        let graph = ConceptGraph::new();
        let bridge = BridgeRetrieval::with_defaults(encoder, graph);
        let singularity = Singularity::new(SingularityConfig::default());

        let results = bridge.query("_default", &singularity, "test query", 10, None).unwrap();
        assert!(results.is_empty());
    }
}
