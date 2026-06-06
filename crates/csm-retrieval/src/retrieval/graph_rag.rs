#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Combines vector similarity with graph traversal for unified retrieval.

// Casts are intentional for scoring formula

use csm_core::Result;
use crate::graph_traversal::TraversalConfig;
use csm_core::hyperdim::HVec10240;
use csm_memory::singularity::Concept;
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
struct Candidate {
    id: String,
    anchor_id: String,
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

    let concept_map: HashMap<String, &Concept> =
        concepts.iter().map(|c| (c.id.clone(), c)).collect();

    let assoc_map: HashMap<String, Vec<(String, f32)>> = {
        let mut map: HashMap<String, Vec<(String, f32)>> = HashMap::new();
        for (from, to, strength) in associations {
            map.entry(from.clone())
                .or_default()
                .push((to.clone(), *strength));
        }
        map
    };

    let anchors = find_anchors(query, &concept_map, config.anchor_top_k);
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (anchor_id, _anchor_sim) in &anchors {
        seen.insert(anchor_id.clone());
        candidates.push(Candidate {
            id: anchor_id.clone(),
            anchor_id: anchor_id.clone(),
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
            if seen.contains(&node_id) {
                continue;
            }
            seen.insert(node_id.clone());

            candidates.push(Candidate {
                id: node_id,
                anchor_id: anchor_id.clone(),
                hop_distance: hop,
                path_strength,
            });
        }
    }

    let mut best_by_id: HashMap<String, GraphRagResult> = HashMap::new();

    for candidate in &candidates {
        let concept = match concept_map.get(&candidate.id) {
            Some(c) => c,
            None => continue,
        };

        let similarity = query.cosine_similarity(&concept.vector);
        let graph_score = config.graph_weight
            * (1.0 / (1.0 + candidate.hop_distance as f32))
            * candidate.path_strength;
        let sim_score = config.similarity_weight * similarity;
        let combined = sim_score + graph_score;

        let result = GraphRagResult {
            id: candidate.id.clone(),
            score: combined,
            similarity,
            anchor_id: Some(candidate.anchor_id.clone()),
            hop_distance: candidate.hop_distance,
            assoc_strength: candidate.path_strength,
        };
        if best_by_id
            .get(&candidate.id)
            .is_none_or(|e| e.score < combined)
        {
            best_by_id.insert(candidate.id.clone(), result);
        }
    }

    let mut results: Vec<GraphRagResult> = best_by_id.values().cloned().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(config.final_top_k);

    Ok(results)
}

/// Find anchor concepts via brute-force similarity.
fn find_anchors(
    query: &HVec10240,
    concepts: &HashMap<String, &Concept>,
    top_k: usize,
) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = concepts
        .iter()
        .map(|(id, c)| (id.clone(), query.cosine_similarity(&c.vector)))
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(top_k);
    scored
}

/// BFS traverse from a starting concept.
///
/// Re-visits nodes if a path with a higher graph score (strength / (1 + hops)) is found.
fn traverse_from(
    start: &str,
    associations: &HashMap<String, Vec<(String, f32)>>,
    config: &TraversalConfig,
) -> Vec<(String, usize, f32)> {
    // Map of node_id -> (depth, path_strength, graph_score)
    let mut best_paths: HashMap<String, (usize, f32, f32)> = HashMap::new();
    let mut queue: VecDeque<(String, usize, f32)> = VecDeque::new();

    queue.push_back((start.to_string(), 0, 1.0));
    best_paths.insert(start.to_string(), (0, 1.0, 1.0));

    while let Some((current, depth, path_strength)) = queue.pop_front() {
        if depth >= config.max_depth {
            continue;
        }

        if let Some(edges) = associations.get(&current) {
            for (neighbor, strength) in edges {
                if *strength < config.min_strength {
                    continue;
                }

                let new_depth = depth + 1;
                let new_strength = path_strength.min(*strength);
                let new_graph_score = new_strength / (1.0 + new_depth as f32);

                let is_better = if let Some(&(_, _, prev_score)) = best_paths.get(neighbor) {
                    new_graph_score > prev_score
                } else {
                    // Start node is in best_paths but not in results
                    (best_paths.len() - 1) < config.max_results
                };

                if is_better {
                    best_paths.insert(neighbor.clone(), (new_depth, new_strength, new_graph_score));
                    queue.push_back((neighbor.clone(), new_depth, new_strength));
                }
            }
        }
    }

    best_paths
        .into_iter()
        .filter(|(id, _)| id != start)
        .map(|(id, (depth, strength, _))| (id, depth, strength))
        .collect()
}
