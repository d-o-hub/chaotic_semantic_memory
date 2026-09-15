//! Semantic Bridge Layer for Zero-Drift Semantic Generalization.
//!
//! This module provides types for a symbolic semantic expansion layer that sits
//! on top of the deterministic HDC memory system without introducing embedding drift.
//!
//! See ADR-0061 for architecture decisions.

// Casts are intentional for bridge version math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use serde::{Deserialize, Serialize};

pub use csm_traits::{CanonicalConcept, ConceptGraph};

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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

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
