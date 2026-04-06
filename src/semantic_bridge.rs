//! Semantic Bridge Layer for Zero-Drift Semantic Generalization.
//!
//! This module provides types for a symbolic semantic expansion layer that sits
//! on top of the deterministic HDC memory system without introducing embedding drift.
//!
//! See ADR-0061 for architecture decisions.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// A canonical concept with symbolic identity relationships.
///
/// Canonical concepts represent semantic equivalence classes (e.g., "agent memory"
/// ≈ "cross-session context" ≈ "ai memory") without modifying stored vectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalConcept {
    /// Unique identifier (e.g., "concept.agent_memory").
    pub id: String,
    /// Version for tracking changes.
    pub version: u32,
    /// Human-readable labels/aliases for this concept.
    pub labels: Vec<String>,
    /// Related concept IDs (symbolic relationships, not similarity).
    pub related: Vec<String>,
}

impl CanonicalConcept {
    /// Create a new canonical concept with the given ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: 1,
            labels: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Add a label/alias to this concept.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Add a related concept ID.
    pub fn with_related(mut self, related_id: impl Into<String>) -> Self {
        self.related.push(related_id.into());
        self
    }
}

/// Configuration for the bridge retrieval pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Maximum depth for concept expansion.
    pub max_expansion_depth: u8,
    /// Maximum number of facts in a memory packet.
    pub max_packet_facts: usize,
    /// Token budget for memory packet compression.
    pub token_budget: usize,
    /// Weight for deterministic score in final score calculation.
    pub deterministic_weight: f32,
    /// Weight for concept expansion score.
    pub concept_weight: f32,
    /// Weight for semantic reranker score.
    pub semantic_weight: f32,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            max_expansion_depth: 2,
            max_packet_facts: 20,
            token_budget: 1000,
            deterministic_weight: 0.6,
            concept_weight: 0.3,
            semantic_weight: 0.1,
        }
    }
}

/// Breakdown of scores for a bridge retrieval hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Deterministic HDC similarity score.
    pub deterministic: f32,
    /// Concept expansion match score.
    pub concept: f32,
    /// Semantic reranker score (0.0 if no reranker).
    pub semantic: f32,
    /// Final combined score.
    pub final_score: f32,
    /// Evidence trail for this hit.
    pub evidence: Vec<String>,
}

impl ScoreBreakdown {
    /// Create a new score breakdown with deterministic score only.
    pub fn deterministic_only(score: f32) -> Self {
        Self {
            deterministic: score,
            concept: 0.0,
            semantic: 0.0,
            final_score: score,
            evidence: vec!["deterministic_recall".to_string()],
        }
    }
}

/// A single hit from bridge retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHit {
    /// Concept ID.
    pub id: String,
    /// Text preview (if available).
    pub text_preview: Option<String>,
    /// Score breakdown.
    pub scores: ScoreBreakdown,
}

/// Compressed memory packet for LLM context injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPacket {
    /// The original query intent.
    pub query_intent: String,
    /// Extracted facts from retrieval.
    pub facts: Vec<String>,
    /// Source concept IDs.
    pub sources: Vec<String>,
    /// Overall confidence score.
    pub confidence: f32,
}

impl MemoryPacket {
    /// Estimate token count using word-count heuristic.
    pub fn estimated_tokens(&self) -> usize {
        let word_count: usize = self
            .facts
            .iter()
            .map(|f| f.split_whitespace().count())
            .sum();
        // Heuristic: words / 0.75 tokens per word average
        (word_count as f32 / 0.75).ceil() as usize
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON string.
    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

/// Trait for optional semantic reranking.
///
/// Implementations can wrap local models, remote APIs, or rule-based heuristics.
/// The reranker never mutates deterministic scores—only adjusts ordering.
pub trait SemanticReranker: Send + Sync {
    /// Return a version string for this reranker.
    fn version(&self) -> &str;

    /// Rerank hits based on query semantics.
    /// Implementations should update `scores.semantic` and `scores.final_score`.
    fn rerank(&self, query: &str, hits: &mut [BridgeHit]);
}

/// In-memory canonical concept graph for symbolic semantic expansion.
#[derive(Debug, Clone, Default)]
pub struct ConceptGraph {
    /// Concepts indexed by ID.
    concepts: std::collections::HashMap<String, CanonicalConcept>,
    /// Label → concept IDs index (lowercased for case-insensitive matching).
    label_index: std::collections::HashMap<String, Vec<String>>,
}

impl ConceptGraph {
    /// Create an empty concept graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a concept to the graph, indexing all labels.
    pub fn add_concept(&mut self, concept: CanonicalConcept) {
        let id = concept.id.clone();
        let labels: Vec<String> = concept.labels.iter().map(|l| l.to_lowercase()).collect();

        // Index labels
        for label in &labels {
            self.label_index
                .entry(label.clone())
                .or_default()
                .push(id.clone());
        }

        self.concepts.insert(id, concept);
    }

    /// Remove a concept and clean up its label index entries.
    pub fn remove_concept(&mut self, id: &str) -> Option<CanonicalConcept> {
        let concept = self.concepts.remove(id)?;
        // Clean up label index
        for label in &concept.labels {
            if let Some(ids) = self.label_index.get_mut(&label.to_lowercase()) {
                ids.retain(|i| i != id);
                if ids.is_empty() {
                    self.label_index.remove(&label.to_lowercase());
                }
            }
        }
        Some(concept)
    }

    /// Get a concept by ID.
    pub fn get_concept(&self, id: &str) -> Option<&CanonicalConcept> {
        self.concepts.get(id)
    }

    /// Match tokens to concept IDs via the label index.
    pub fn match_tokens(&self, tokens: &[String]) -> Vec<String> {
        let mut matched = std::collections::HashSet::new();
        for token in tokens {
            if let Some(ids) = self.label_index.get(&token.to_lowercase()) {
                matched.extend(ids.clone());
            }
        }
        matched.into_iter().collect()
    }

    /// Expand concept IDs to their labels and related concept labels.
    pub fn expand(&self, concept_ids: &[String], max_depth: u8) -> Vec<String> {
        let mut expanded = std::collections::HashSet::new();
        let mut to_visit: Vec<(String, u8)> =
            concept_ids.iter().map(|id| (id.clone(), 0)).collect();
        let mut visited = std::collections::HashSet::new();

        while let Some((id, depth)) = to_visit.pop() {
            if visited.contains(&id) || depth > max_depth {
                continue;
            }
            visited.insert(id.clone());

            if let Some(concept) = self.concepts.get(&id) {
                // Add all labels
                for label in &concept.labels {
                    expanded.insert(label.clone());
                }
                // Queue related concepts
                if depth < max_depth {
                    for related_id in &concept.related {
                        if !visited.contains(related_id) {
                            to_visit.push((related_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        expanded.into_iter().collect()
    }

    /// Load concept graph from JSON.
    pub fn load_from_json(reader: impl Read) -> crate::Result<Self> {
        let concepts: Vec<CanonicalConcept> = serde_json::from_reader(reader)?;
        let mut graph = Self::new();
        for concept in concepts {
            graph.add_concept(concept);
        }
        Ok(graph)
    }

    /// Save concept graph to JSON.
    pub fn save_to_json(&self, writer: impl Write) -> crate::Result<()> {
        let concepts: Vec<&CanonicalConcept> = self.concepts.values().collect();
        serde_json::to_writer_pretty(writer, &concepts)?;
        Ok(())
    }

    /// Return the number of concepts in the graph.
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Return the number of unique labels in the index.
    pub fn label_count(&self) -> usize {
        self.label_index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_concept_builder() {
        let concept = CanonicalConcept::new("concept.test")
            .with_label("test label")
            .with_related("concept.other");
        assert_eq!(concept.id, "concept.test");
        assert_eq!(concept.labels, vec!["test label"]);
        assert_eq!(concept.related, vec!["concept.other"]);
    }

    #[test]
    fn test_concept_graph_add_and_get() {
        let mut graph = ConceptGraph::new();
        let concept = CanonicalConcept::new("c1")
            .with_label("label1")
            .with_label("Label2"); // Different case
        graph.add_concept(concept);

        assert_eq!(graph.concept_count(), 1);
        assert_eq!(graph.label_count(), 2); // "label1" and "label2"
        assert!(graph.get_concept("c1").is_some());
    }

    #[test]
    fn test_concept_graph_match_tokens_case_insensitive() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1").with_label("agent-memory"), // Single token (lowercased)
        );
        graph.add_concept(CanonicalConcept::new("c2").with_label("session"));

        let matched = graph.match_tokens(&["Agent-Memory".to_string(), "SESSION".to_string()]);
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn test_concept_graph_expand() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("agent memory")
                .with_related("c2"),
        );
        graph.add_concept(CanonicalConcept::new("c2").with_label("session context"));

        let expanded = graph.expand(&["c1".to_string()], 1);
        assert!(expanded.contains(&"agent memory".to_string()));
        assert!(expanded.contains(&"session context".to_string()));
    }

    #[test]
    fn test_concept_graph_expand_no_cycle() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("label1")
                .with_related("c2"),
        );
        graph.add_concept(
            CanonicalConcept::new("c2")
                .with_label("label2")
                .with_related("c1"), // Cycle!
        );

        // Should not infinite loop
        let expanded = graph.expand(&["c1".to_string()], 10);
        assert!(expanded.contains(&"label1".to_string()));
        assert!(expanded.contains(&"label2".to_string()));
    }

    #[test]
    fn test_concept_graph_remove() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(CanonicalConcept::new("c1").with_label("label1"));
        assert_eq!(graph.label_count(), 1);

        graph.remove_concept("c1");
        assert_eq!(graph.concept_count(), 0);
        assert_eq!(graph.label_count(), 0); // Label cleaned up
    }

    #[test]
    fn test_memory_packet_estimated_tokens() {
        let packet = MemoryPacket {
            query_intent: "test".to_string(),
            facts: vec!["hello world".to_string(), "foo bar baz".to_string()],
            sources: vec!["c1".to_string()],
            confidence: 0.9,
        };
        // 5 words / 0.75 = ~7 tokens
        assert!(packet.estimated_tokens() >= 5);
    }

    #[test]
    fn test_bridge_config_defaults() {
        let config = BridgeConfig::default();
        assert_eq!(config.max_expansion_depth, 2);
        assert_eq!(config.max_packet_facts, 20);
        assert!((config.deterministic_weight - 0.6).abs() < 0.01);
    }
}
