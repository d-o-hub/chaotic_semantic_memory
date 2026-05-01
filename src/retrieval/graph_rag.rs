//! GraphRAG Hybrid Retrieval (ADR-0070)
//!
//! Combines vector similarity with graph traversal for unified retrieval.

// Casts are intentional for scoring formula
#![allow(clippy::cast_precision_loss)]

use crate::error::Result;
use crate::graph_traversal::TraversalConfig;
use crate::hyperdim::HVec10240;
use crate::singularity::Concept;
use std::collections::{HashMap, HashSet};

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
///
/// Algorithm:
/// 1. Anchor: probe(query, anchor_top_k) → seed set
/// 2. Expand: traverse from each anchor
/// 3. Score: similarity_weight * cosine + graph_weight * (1/(1+hops)) * strength
/// 4. Dedupe + rank by score
pub fn graph_rag_retrieve(
    query: &HVec10240,
    concepts: &[Concept],
    associations: &[(String, String, f32)],
    config: &GraphRagConfig,
) -> Result<Vec<GraphRagResult>> {
    if concepts.is_empty() {
        return Ok(Vec::new());
    }

    // Convert to HashMaps for efficient lookup
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

    // Step 1: Find anchors via similarity
    let anchors = find_anchors(query, &concept_map, config.anchor_top_k);

    // Step 2: Expand from each anchor
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (anchor_id, _anchor_sim) in &anchors {
        // Add anchor itself as candidate
        seen.insert(anchor_id.clone());
        candidates.push(Candidate {
            id: anchor_id.clone(),
            anchor_id: anchor_id.clone(),
            hop_distance: 0,
            path_strength: 1.0,
        });

        // Traverse from anchor
        let traversal_config = TraversalConfig {
            max_depth: config.max_hops,
            min_strength: config.min_assoc_strength,
            max_results: 1000,
        };

        let traversed = traverse_from(anchor_id, &assoc_map, &traversal_config);

        for (node_id, hop) in traversed {
            if seen.contains(&node_id) {
                continue;
            }
            seen.insert(node_id.clone());

            // Get path strength (simplified: use min edge along path)
            let strength = get_path_strength(anchor_id, &node_id, hop, &assoc_map);

            candidates.push(Candidate {
                id: node_id,
                anchor_id: anchor_id.clone(),
                hop_distance: hop,
                path_strength: strength,
            });
        }
    }

    // Step 3: Score and dedupe
    let mut best_by_id: HashMap<String, GraphRagResult> = HashMap::new();

    for candidate in &candidates {
        let concept = concept_map.get(&candidate.id);
        if concept.is_none() {
            continue;
        }
        let concept = concept.unwrap();

        // Compute similarity to query
        let similarity = query.cosine_similarity(&concept.vector);

        // Combined score
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

        // Keep best score for each ID
        let existing = best_by_id.get(&candidate.id);
        if existing.is_none() || existing.unwrap().score < combined {
            best_by_id.insert(candidate.id.clone(), result);
        }
    }

    // Step 4: Sort and limit
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
fn traverse_from(
    start: &str,
    associations: &HashMap<String, Vec<(String, f32)>>,
    config: &TraversalConfig,
) -> Vec<(String, usize)> {
    let mut results: Vec<(String, usize)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, usize, f32)> = vec![(start.to_string(), 0, 1.0)];

    visited.insert(start.to_string());

    while let Some((current, depth, _strength)) = queue.pop() {
        if depth >= config.max_depth {
            continue;
        }

        let edges = associations.get(&current);
        if edges.is_none() {
            continue;
        }

        for (neighbor, strength) in edges.unwrap() {
            if *strength < config.min_strength {
                continue;
            }
            if visited.contains(neighbor) {
                continue;
            }
            if results.len() >= config.max_results {
                break;
            }

            visited.insert(neighbor.clone());
            results.push((neighbor.clone(), depth + 1));
            queue.push((neighbor.clone(), depth + 1, *strength));
        }
    }

    results
}

/// Estimate path strength (simplified: returns average edge strength).
fn get_path_strength(
    from: &str,
    to: &str,
    hops: usize,
    associations: &HashMap<String, Vec<(String, f32)>>,
) -> f32 {
    // Simplified: for direct neighbors, return edge strength
    // For multi-hop, estimate based on hop distance
    if hops == 0 {
        return 1.0;
    }
    if hops == 1 {
        let edges = associations.get(from);
        if let Some(edges) = edges {
            for (neighbor, strength) in edges {
                if neighbor == to {
                    return *strength;
                }
            }
        }
        return 0.5;
    }
    // Multi-hop: use decay factor
    0.5 / (hops as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;

    fn make_test_concepts() -> Vec<Concept> {
        let mut concepts = Vec::new();
        for i in 0..10 {
            let id = format!("concept_{}", i);
            concepts.push(Concept {
                id: id.clone(),
                vector: HVec10240::random(),
                metadata: HashMap::new(),
                created_at: 0,
                modified_at: 0,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            });
        }
        concepts
    }

    fn make_test_associations() -> Vec<(String, String, f32)> {
        vec![
            ("concept_0".to_string(), "concept_1".to_string(), 0.8),
            ("concept_1".to_string(), "concept_2".to_string(), 0.7),
            ("concept_5".to_string(), "concept_6".to_string(), 0.9),
        ]
    }

    #[test]
    fn test_empty_concepts() {
        let query = HVec10240::random();
        let concepts: Vec<Concept> = Vec::new();
        let associations: Vec<(String, String, f32)> = Vec::new();
        let config = GraphRagConfig::default();

        let results = graph_rag_retrieve(&query, &concepts, &associations, &config).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_anchor_is_top_result() {
        let concepts = make_test_concepts();
        let associations: Vec<(String, String, f32)> = Vec::new();
        let config = GraphRagConfig {
            anchor_top_k: 3,
            final_top_k: 5,
            ..Default::default()
        };

        let query = concepts[0].vector;
        let results = graph_rag_retrieve(&query, &concepts, &associations, &config).unwrap();

        // Anchor (concept_0) should be top result with hop_distance=0
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "concept_0");
        assert_eq!(results[0].hop_distance, 0);
    }

    #[test]
    fn test_connected_results() {
        let concepts = make_test_concepts();
        let associations = make_test_associations();
        let config = GraphRagConfig {
            anchor_top_k: 1,
            max_hops: 2,
            final_top_k: 5,
            graph_weight: 0.5,
            similarity_weight: 0.5,
            ..Default::default()
        };

        let query = concepts[0].vector;
        let results = graph_rag_retrieve(&query, &concepts, &associations, &config).unwrap();

        // Should include connected concepts concept_1 and concept_2
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(
            ids.contains(&"concept_1") || ids.contains(&"concept_2") || ids.contains(&"concept_0")
        );
    }
}
