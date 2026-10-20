#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Combines vector similarity with graph traversal for unified retrieval.

// Casts are intentional for scoring formula

use csm_core_lib::error::{MemoryError, Result};
use csm_core_lib::hyperdim::HVec10240;
use csm_memory::Concept;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};

/// Configuration for GraphRAG retrieval.
#[derive(Debug, Clone)]
pub struct GraphRagConfig {
    /// Number of anchor concepts from initial probe.
    pub anchor_top_k: usize,
    /// Maximum hops to traverse from each anchor.
    pub max_hops: usize,
    /// Minimum association strength to follow.
    pub min_assoc_strength: f32,
    /// Weight for similarity score component (0.0-1.0).
    pub similarity_weight: f32,
    /// Weight for graph distance component (0.0-1.0).
    pub graph_weight: f32,
    /// Final top-K results to return.
    pub final_top_k: usize,
}

impl Default for GraphRagConfig {
    fn default() -> Self {
        Self {
            anchor_top_k: 5,
            max_hops: 2,
            min_assoc_strength: 0.0,
            similarity_weight: 0.6,
            graph_weight: 0.4,
            final_top_k: 20,
        }
    }
}

/// Result from GraphRAG retrieval.
#[derive(Debug, Clone)]
pub struct GraphRagResult {
    /// Concept ID.
    pub id: String,
    /// Combined score (similarity + graph).
    pub score: f32,
    /// Raw cosine similarity to query.
    pub similarity: f32,
    /// Anchor concept that reached this result.
    pub anchor_id: Option<String>,
    /// Hop distance from anchor (0 = anchor itself).
    pub hop_distance: usize,
    /// Association strength along path.
    pub assoc_strength: f32,
}

/// Hard bounds for GraphRAG retrieval parameters (CWE-770).
pub const MAX_GRAPH_RAG_ANCHOR_TOP_K: usize = 10_000;
pub const MAX_GRAPH_RAG_HOPS: usize = 32;
pub const MAX_GRAPH_RAG_FINAL_TOP_K: usize = 100_000;

/// Maximum graph traversal expanded candidates limit.
const MAX_TRAVERSAL_EXPANSIONS: usize = 1000;

fn validate_graph_rag_config(config: &GraphRagConfig) -> Result<()> {
    for (field, value) in [
        ("min_assoc_strength", config.min_assoc_strength),
        ("similarity_weight", config.similarity_weight),
        ("graph_weight", config.graph_weight),
    ] {
        if !value.is_finite() {
            return Err(MemoryError::InvalidInput {
                field: field.to_string(),
                reason: "must be finite".to_string(),
            });
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(MemoryError::InvalidInput {
                field: field.to_string(),
                reason: format!("must be in [0.0, 1.0], got {value}"),
            });
        }
    }
    for (field, value, maximum) in [
        (
            "anchor_top_k",
            config.anchor_top_k,
            MAX_GRAPH_RAG_ANCHOR_TOP_K,
        ),
        ("max_hops", config.max_hops, MAX_GRAPH_RAG_HOPS),
        ("final_top_k", config.final_top_k, MAX_GRAPH_RAG_FINAL_TOP_K),
    ] {
        if value > maximum {
            return Err(MemoryError::InvalidInput {
                field: field.to_string(),
                reason: format!("must not exceed {maximum}, got {value}"),
            });
        }
    }
    Ok(())
}

/// Execute GraphRAG retrieval.
pub fn graph_rag_retrieve(
    query: &HVec10240,
    concepts: &[Concept],
    associations: &[(String, String, f32)],
    config: &GraphRagConfig,
) -> Result<Vec<GraphRagResult>> {
    validate_graph_rag_config(config)?;
    let anchor_top_k = config.anchor_top_k;
    let max_hops = config.max_hops;
    let final_top_k = config.final_top_k;

    if concepts.is_empty() {
        return Ok(Vec::new());
    }
    if final_top_k == 0 {
        return Ok(Vec::new());
    }

    // Optimization: Merge similarity calculation, anchor collection, and map construction
    // into fewer passes to improve cache locality and minimize reallocations.
    let mut scored_anchors = Vec::with_capacity(concepts.len());
    let mut concept_map = HashMap::with_capacity(concepts.len());

    for c in concepts {
        let sim = query.cosine_similarity(&c.vector);
        let id = c.id.as_str();
        scored_anchors.push((id, sim));
        concept_map.insert(id, (c, sim));
    }

    let anchors = {
        let top_k = anchor_top_k;
        if top_k == 0 {
            Vec::new()
        } else {
            if scored_anchors.len() > top_k {
                scored_anchors.select_nth_unstable_by(top_k - 1, |a, b| b.1.total_cmp(&a.1));
                scored_anchors.truncate(top_k);
            }
            scored_anchors.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
            scored_anchors
        }
    };

    let assoc_map: HashMap<&str, Vec<(&str, f32)>> = {
        let mut map: HashMap<&str, Vec<(&str, f32)>> = HashMap::with_capacity(associations.len());
        for (from, to, strength) in associations {
            map.entry(from.as_str())
                .or_default()
                .push((to.as_str(), *strength));
        }
        map
    };

    // Optimization: Unified multi-source BFS starting from all anchors.
    // This ensures nodes are reached via their optimal paths from any anchor and avoids
    // redundant traversal overhead from multiple single-source BFS calls.
    // Map: id -> (best_score, similarity, anchor_id, hop, path_strength)
    let mut results_map: HashMap<&str, (f32, f32, &str, usize, f32)> =
        HashMap::with_capacity(anchors.len() * 4);
    let mut queue = VecDeque::with_capacity(anchors.len() * 2);

    for &(anchor_id, sim) in &anchors {
        let graph_score = config.graph_weight * 1.0; // depth 0, strength 1.0
        let total_score = config.similarity_weight * sim + graph_score;

        results_map.insert(anchor_id, (total_score, sim, anchor_id, 0, 1.0));
        queue.push_back((anchor_id, anchor_id, 0, 1.0f32));
    }

    let anchor_count = results_map.len();

    while let Some((current_id, anchor_id, hop, path_strength)) = queue.pop_front() {
        if hop >= max_hops {
            continue;
        }

        if let Some(neighbors) = assoc_map.get(current_id) {
            for &(neighbor_id, edge_strength) in neighbors {
                if edge_strength < config.min_assoc_strength {
                    continue;
                }

                let new_hop = hop + 1;
                let new_strength = path_strength.min(edge_strength);

                let &(_, similarity) = match concept_map.get(neighbor_id) {
                    Some(c) => c,
                    None => continue,
                };

                let graph_score =
                    config.graph_weight * (1.0 / (1.0 + new_hop as f32)) * new_strength;
                let total_score = config.similarity_weight * similarity + graph_score;

                let expanded_count = results_map.len() - anchor_count;
                match results_map.entry(neighbor_id) {
                    Entry::Occupied(mut entry) => {
                        if total_score > entry.get().0 {
                            entry.insert((
                                total_score,
                                similarity,
                                anchor_id,
                                new_hop,
                                new_strength,
                            ));
                            queue.push_back((neighbor_id, anchor_id, new_hop, new_strength));
                        }
                    }
                    Entry::Vacant(entry) => {
                        if expanded_count < MAX_TRAVERSAL_EXPANSIONS {
                            entry.insert((
                                total_score,
                                similarity,
                                anchor_id,
                                new_hop,
                                new_strength,
                            ));
                            queue.push_back((neighbor_id, anchor_id, new_hop, new_strength));
                        }
                    }
                }
            }
        }
    }

    let mut results: Vec<GraphRagResult> = results_map
        .into_iter()
        .map(
            |(id, (score, similarity, anchor_id, hop, path_strength))| GraphRagResult {
                id: id.to_string(),
                score,
                similarity,
                anchor_id: Some(anchor_id.to_string()),
                hop_distance: hop,
                assoc_strength: path_strength,
            },
        )
        .collect();

    // Optimization: Use O(N) selection for top-K results to avoid full O(N log N) sort.
    if results.len() > final_top_k {
        let k = final_top_k;
        results.select_nth_unstable_by(k - 1, |a, b| b.score.total_cmp(&a.score));
        results.truncate(k);
    }
    results.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));

    Ok(results)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use csm_core_lib::error::MemoryError;

    fn concept(id: impl Into<String>, vector: HVec10240) -> Concept {
        Concept {
            id: id.into(),
            vector,
            ..Default::default()
        }
    }

    #[test]
    fn graph_rag_rejects_non_finite_and_out_of_range_weights() {
        let query = HVec10240::random();
        let concepts = vec![concept("c1", HVec10240::random())];
        for (field, value) in [
            ("min_assoc_strength", f32::NAN),
            ("similarity_weight", f32::INFINITY),
            ("graph_weight", -0.1),
        ] {
            let config = match field {
                "min_assoc_strength" => GraphRagConfig {
                    min_assoc_strength: value,
                    ..GraphRagConfig::default()
                },
                "similarity_weight" => GraphRagConfig {
                    similarity_weight: value,
                    ..GraphRagConfig::default()
                },
                _ => GraphRagConfig {
                    graph_weight: value,
                    ..GraphRagConfig::default()
                },
            };
            assert!(matches!(
                graph_rag_retrieve(&query, &concepts, &[], &config),
                Err(MemoryError::InvalidInput { field: actual, .. }) if actual == field
            ));
        }
    }

    #[test]
    fn graph_rag_rejects_integer_limits_above_public_bounds() {
        let query = HVec10240::random();
        let concepts = vec![concept("c1", HVec10240::random())];
        for config in [
            GraphRagConfig {
                anchor_top_k: MAX_GRAPH_RAG_ANCHOR_TOP_K + 1,
                ..GraphRagConfig::default()
            },
            GraphRagConfig {
                max_hops: MAX_GRAPH_RAG_HOPS + 1,
                ..GraphRagConfig::default()
            },
            GraphRagConfig {
                final_top_k: MAX_GRAPH_RAG_FINAL_TOP_K + 1,
                ..GraphRagConfig::default()
            },
        ] {
            assert!(matches!(
                graph_rag_retrieve(&query, &concepts, &[], &config),
                Err(MemoryError::InvalidInput { .. })
            ));
        }
    }

    #[test]
    fn graph_rag_hops_stop_at_the_public_cap() {
        let anchor_vector = HVec10240::random();
        let concepts: Vec<Concept> = (0..=40)
            .map(|index| {
                concept(
                    format!("c{index}"),
                    if index == 0 {
                        anchor_vector
                    } else {
                        HVec10240::random()
                    },
                )
            })
            .collect();
        let associations: Vec<(String, String, f32)> = (0..40)
            .map(|index| (format!("c{index}"), format!("c{}", index + 1), 1.0))
            .collect();
        let config = GraphRagConfig {
            anchor_top_k: 1,
            max_hops: MAX_GRAPH_RAG_HOPS,
            min_assoc_strength: 0.0,
            similarity_weight: 0.6,
            graph_weight: 0.4,
            final_top_k: 100,
        };

        let results = graph_rag_retrieve(&anchor_vector, &concepts, &associations, &config)
            .expect("valid GraphRAG config should succeed");
        assert_eq!(
            results.iter().map(|result| result.hop_distance).max(),
            Some(MAX_GRAPH_RAG_HOPS)
        );
    }

    #[test]
    fn graph_rag_zero_final_top_k_returns_empty() {
        let query = HVec10240::random();
        let concepts = vec![concept("c1", HVec10240::random())];
        let config = GraphRagConfig {
            final_top_k: 0,
            ..GraphRagConfig::default()
        };

        assert!(
            graph_rag_retrieve(&query, &concepts, &[], &config)
                .expect("zero top-k is valid")
                .is_empty()
        );
    }
}
