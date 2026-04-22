//! Semantic Triples Memory Layer
//!
//! Based on "Memori: A Persistent Memory Layer for Efficient, Context-Aware LLM Agents" (2026).
//! This module provides a structured representation of unstructured dialogue
//! into compact semantic triples (Subject, Predicate, Object) to drastically
//! reduce token overhead and improve retrieval precision.


use serde::{Deserialize, Serialize};

/// A semantic triple representing a discrete fact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SemanticTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

impl SemanticTriple {
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        }
    }

    /// Converts the triple into a natural language representation
    pub fn to_sentence(&self) -> String {
        format!("{} {} {}", self.subject, self.predicate, self.object)
    }
}

/// A structured memory payload that can be stored in a `Concept`'s metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMemoryPayload {
    pub raw_summary: String,
    pub triples: Vec<SemanticTriple>,
}

impl StructuredMemoryPayload {
    /// Formats the payload into a compact representation for LLM context windows.
    pub fn to_context_string(&self) -> String {
        let mut out = format!("Summary: {}\nFacts:\n", self.raw_summary);
        for triple in &self.triples {
            out.push_str(&format!("- {}\n", triple.to_sentence()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_formatting() {
        let triple = SemanticTriple::new("User", "prefers", "dark mode");
        assert_eq!(triple.to_sentence(), "User prefers dark mode");
    }

    #[test]
    fn test_payload_formatting() {
        let payload = StructuredMemoryPayload {
            raw_summary: "User preferences regarding UI".to_string(),
            triples: vec![
                SemanticTriple::new("User", "prefers", "dark mode"),
                SemanticTriple::new("User", "uses", "mobile app"),
            ]
        };

        let expected = "Summary: User preferences regarding UI\nFacts:\n- User prefers dark mode\n- User uses mobile app\n";
        assert_eq!(payload.to_context_string(), expected);
    }
}
