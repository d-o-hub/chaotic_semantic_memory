#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Combines vector similarity with graph traversal for unified retrieval.

// Casts are intentional for scoring formula

use csm_core::error::Result;
use csm_core::hyperdim::HVec10240;
use csm_memory::{Concept, TraversalConfig};
use std::collections::{HashMap, HashSet, VecDeque};

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

/// Internal candidate during GraphRAG expansion.
#[derive(Debug, Clone)]
struct Candidate<'a> {
    id: &'a str,
    anchor_id: &'a str,
    hop_distance: usize,
    path_strength: f32,
}

/// Execute GraphRAG retrieval.
pub fn graph_rag_retrieve(
    query: &HVec10240,
    concepts: &[Concept],
    associations: &[(String, String, f32)],
    config: &GraphRagConfig,
) -> Result<Vec<GraphRagResult>> {
    if concepts.is_empty() {
        return Ok(Vec::new());
    }

    // Optimization: Calculate similarities upfront.
    // While batch_cosine_similarity exists, it requires contiguous HVec10240.
    // HVec10240::cosine_similarity is already SIMD-accelerated.
    let similarities: Vec<f32> = concepts
        .iter()
        .map(|c| query.cosine_similarity(&c.vector))
        .collect();

    // Build efficient lookup maps using &str keys to avoid String allocations.
    let concept_map: HashMap<&str, (&Concept, f32)> = concepts
        .iter()
        .zip(similarities.iter())
        .map(|(c, &sim)| (c.id.as_str(), (c, sim)))
        .collect();

    let assoc_map: HashMap<&str, Vec<(&str, f32)>> = {
        let mut map: HashMap<&str, Vec<(&str, f32)>> = HashMap::with_capacity(associations.len());
        for (from, to, strength) in associations {
            map.entry(from.as_str())
                .or_default()
                .push((to.as_str(), *strength));
        }
        map
    };

    let anchors = find_anchors(&concept_map, config.anchor_top_k);
    let mut candidates: Vec<Candidate> = Vec::with_capacity(anchors.len() * 2);
    let mut seen: HashSet<&str> = HashSet::with_capacity(concepts.len());

    for (anchor_id, _anchor_sim) in &anchors {
        seen.insert(anchor_id);
        candidates.push(Candidate {
            id: anchor_id,
            anchor_id,
            hop_distance: 0,
            path_strength: 1.0,
        });

        let traversal_config = TraversalConfig {
            max_depth: config.max_hops,
            min_strength: config.min_assoc_strength,
            max_results: 1000,
        };

        let traversed = traverse_from(anchor_id, &assoc_map, &traversal_config);

        for (node_id, hop, path_strength) in traversed {
            if seen.contains(node_id) {
                continue;
            }
            seen.insert(node_id);

            candidates.push(Candidate {
                id: node_id,
                anchor_id,
                hop_distance: hop,
                path_strength,
            });
        }
    }

    let mut best_by_id: HashMap<&str, GraphRagResult> = HashMap::with_capacity(candidates.len());

    for candidate in &candidates {
        let &(_, similarity) = match concept_map.get(candidate.id) {
            Some(c) => c,
            None => continue,
        };

        // Mathematical Impact: O(1) score calculation using pre-calculated similarity.
        let graph_score = config.graph_weight
            * (1.0 / (1.0 + candidate.hop_distance as f32))
            * candidate.path_strength;
        let sim_score = config.similarity_weight * similarity;
        let combined = sim_score + graph_score;

        if best_by_id
            .get(candidate.id)
            .is_none_or(|e| e.score < combined)
        {
            best_by_id.insert(
                candidate.id,
                GraphRagResult {
                    id: candidate.id.to_string(),
                    score: combined,
                    similarity,
                    anchor_id: Some(candidate.anchor_id.to_string()),
                    hop_distance: candidate.hop_distance,
                    assoc_strength: candidate.path_strength,
                },
            );
        }
    }

    let mut results: Vec<GraphRagResult> = best_by_id.into_values().collect();
    // Optimization: Use unstable sort and total_cmp for faster result ranking.
    results.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    results.truncate(config.final_top_k);

    Ok(results)
}

/// Find anchor concepts using pre-calculated similarities.
fn find_anchors<'a>(
    concepts: &HashMap<&'a str, (&Concept, f32)>,
    top_k: usize,
) -> Vec<(&'a str, f32)> {
    let mut scored: Vec<(&str, f32)> = concepts.iter().map(|(&id, &(_, sim))| (id, sim)).collect();

    scored.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(top_k);
    scored
}

/// BFS traverse from a starting concept using &str to eliminate allocations.
fn traverse_from<'a>(
    start: &'a str,
    associations: &HashMap<&'a str, Vec<(&'a str, f32)>>,
    config: &TraversalConfig,
) -> Vec<(&'a str, usize, f32)> {
    // Map of node_id -> (depth, path_strength, graph_score)
    let mut best_paths: HashMap<&str, (usize, f32, f32)> = HashMap::new();
    let mut queue: VecDeque<(&str, usize, f32)> = VecDeque::new();

    queue.push_back((start, 0, 1.0));
    best_paths.insert(start, (0, 1.0, 1.0));

    while let Some((current, depth, path_strength)) = queue.pop_front() {
        if depth >= config.max_depth {
            continue;
        }

        if let Some(edges) = associations.get(current) {
            for &(neighbor, strength) in edges {
                if strength < config.min_strength {
                    continue;
                }

                let new_depth = depth + 1;
                let new_strength = path_strength.min(strength);
                let new_graph_score = new_strength / (1.0 + new_depth as f32);

                let is_better = if let Some(&(_, _, prev_score)) = best_paths.get(neighbor) {
                    new_graph_score > prev_score
                } else {
                    best_paths.len() < config.max_results + 1 // +1 for start node
                };

                if is_better {
                    best_paths.insert(neighbor, (new_depth, new_strength, new_graph_score));
                    queue.push_back((neighbor, new_depth, new_strength));
                }
            }
        }
    }

    best_paths
        .into_iter()
        .filter(|&(id, _)| id != start)
        .map(|(id, (depth, strength, _))| (id, depth, strength))
        .collect()
}
