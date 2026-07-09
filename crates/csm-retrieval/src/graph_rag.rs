#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Combines vector similarity with graph traversal for unified retrieval.

// Casts are intentional for scoring formula

use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;
use csm_memory::Concept;
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

/// Execute GraphRAG retrieval.
pub fn graph_rag_retrieve(
    query: &HVec10240,
    concepts: &[Concept],
    associations: &[(String, String, f32)],
    config: &GraphRagConfig,
) -> Result<Vec<GraphRagResult>> {
    if concepts.is_empty() || config.final_top_k == 0 {
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
        let top_k = config.anchor_top_k;
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

    while let Some((current_id, anchor_id, hop, path_strength)) = queue.pop_front() {
        if hop >= config.max_hops {
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

                use std::collections::hash_map::Entry;
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
                        entry.insert((total_score, similarity, anchor_id, new_hop, new_strength));
                        queue.push_back((neighbor_id, anchor_id, new_hop, new_strength));
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
    if results.len() > config.final_top_k {
        let k = config.final_top_k;
        results.select_nth_unstable_by(k - 1, |a, b| b.score.total_cmp(&a.score));
        results.truncate(k);
    }
    results.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));

    Ok(results)
}
