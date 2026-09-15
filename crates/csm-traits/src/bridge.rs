//! Canonical concept graph shared contracts (ADR-0061, ADR-0094).
//!
//! `CanonicalConcept` and `ConceptGraph` describe the symbolic expansion layer
//! that sits on top of the deterministic HDC memory system. They live here so
//! the persistence crate can implement durable CRUD over them without the root
//! facade owning a second implementation (ADR-0094).

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

        // Index labels
        for label in &concept.labels {
            // Algorithmic Optimization: Avoid intermediate vector allocation and lowercase directly
            self.label_index
                .entry(label.to_lowercase())
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
            // Algorithmic Optimization: Avoid double lookup by reusing the entry
            let lowered = label.to_lowercase();
            if let std::collections::hash_map::Entry::Occupied(mut entry) =
                self.label_index.entry(lowered)
            {
                let ids = entry.get_mut();
                ids.retain(|i| i != id);
                if ids.is_empty() {
                    entry.remove();
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
            // Algorithmic Optimization: Use &str references in the matched set to avoid
            // cloning concept IDs during the matching process.
            if let Some(ids) = self.label_index.get(token) {
                matched.extend(ids.iter().map(|s| s.as_str()));
            } else {
                // If not found, attempt case-insensitive lookup
                let lowered = token.to_lowercase();
                if &lowered != token {
                    if let Some(ids) = self.label_index.get(&lowered) {
                        matched.extend(ids.iter().map(|s| s.as_str()));
                    }
                }
            }
        }
        matched.into_iter().map(|s| s.to_string()).collect()
    }

    /// Expand concept IDs to their labels and related concept labels.
    pub fn expand(&self, concept_ids: &[String], max_depth: u8) -> Vec<String> {
        let mut expanded = std::collections::HashSet::new();
        let mut to_visit: Vec<(&str, u8)> = concept_ids.iter().map(|id| (id.as_str(), 0)).collect();
        let mut visited = std::collections::HashSet::new();

        while let Some((id, depth)) = to_visit.pop() {
            // Algorithmic Optimization: Use insert() result to combine check and mark in one lookup.
            // Using &str references for visited and to_visit sets eliminates redundant String
            // allocations and clones during the graph traversal hot path.
            if !visited.insert(id) {
                continue;
            }

            if let Some(concept) = self.concepts.get(id) {
                // Add all labels
                for label in &concept.labels {
                    expanded.insert(label.as_str());
                }
                // Queue related concepts
                if depth < max_depth {
                    for related_id in &concept.related {
                        if !visited.contains(related_id.as_str()) {
                            to_visit.push((related_id.as_str(), depth + 1));
                        }
                    }
                }
            }
        }

        expanded.into_iter().map(|s| s.to_string()).collect()
    }

    /// Load concept graph from JSON.
    pub fn load_from_json(reader: impl Read) -> csm_core_lib::Result<Self> {
        let concepts: Vec<CanonicalConcept> = serde_json::from_reader(reader)?;
        let mut graph = Self::new();
        for concept in concepts {
            graph.add_concept(concept);
        }
        Ok(graph)
    }

    /// Save concept graph to JSON.
    pub fn save_to_json(&self, writer: impl Write) -> csm_core_lib::Result<()> {
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

    /// Return an iterator over all concepts.
    pub fn all_concepts(&self) -> impl Iterator<Item = &CanonicalConcept> {
        self.concepts.values()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
    fn test_concept_graph_expand_max_depth_strict() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            CanonicalConcept::new("c1")
                .with_label("l1")
                .with_related("c2"),
        );
        graph.add_concept(
            CanonicalConcept::new("c2")
                .with_label("l2")
                .with_related("c3"),
        );
        graph.add_concept(CanonicalConcept::new("c3").with_label("l3"));

        // Max depth 1 should reach c1 and c2, but not c3
        let expanded = graph.expand(&["c1".to_string()], 1);
        assert!(expanded.contains(&"l1".to_string()));
        assert!(expanded.contains(&"l2".to_string()));
        assert!(!expanded.contains(&"l3".to_string()));

        // Max depth 2 should reach all
        let expanded_full = graph.expand(&["c1".to_string()], 2);
        assert!(expanded_full.contains(&"l1".to_string()));
        assert!(expanded_full.contains(&"l2".to_string()));
        assert!(expanded_full.contains(&"l3".to_string()));
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
}
