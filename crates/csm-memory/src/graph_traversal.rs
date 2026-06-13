//! Graph traversal operations on the association graph.
//!
//! Provides BFS, shortest path, and neighbor queries on the concept association graph.
//!
//! # Shortest Path
//!
//! Two variants are provided:
//! - [`Singularity::shortest_path`]: Weighted Dijkstra using `-ln(strength)` as edge cost.
//!   Prefers paths through stronger associations. Returns the minimum-cost path.
//! - [`Singularity::shortest_path_hops`]: Unweighted BFS. Returns the fewest-hop path
//!   regardless of edge strength. Use when hop count matters more than association strength.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use crate::singularity::Singularity;
use csm_core::error::{MemoryError, Result};

/// Maximum traversal depth to prevent excessive resource usage.
const MAX_TRAVERSAL_DEPTH: usize = 32;
/// Maximum traversal results to prevent memory exhaustion.
const MAX_TRAVERSAL_RESULTS: usize = 10_000;

/// Configuration for graph traversal operations.
#[derive(Debug, Clone)]
pub struct TraversalConfig {
    /// Maximum number of hops to traverse.
    pub max_depth: usize,
    /// Minimum edge strength to follow.
    pub min_strength: f32,
    /// Maximum number of nodes to visit.
    pub max_results: usize,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            min_strength: 0.0,
            max_results: 100,
        }
    }
}

impl TraversalConfig {
    /// Validate traversal config parameters.
    pub fn validate(&self) -> Result<()> {
        if self.max_depth > MAX_TRAVERSAL_DEPTH {
            return Err(MemoryError::InvalidInput {
                field: "max_depth".to_string(),
                reason: format!(
                    "traversal depth exceeds {} (got {})",
                    MAX_TRAVERSAL_DEPTH, self.max_depth
                ),
            });
        }
        if self.max_results > MAX_TRAVERSAL_RESULTS {
            return Err(MemoryError::InvalidInput {
                field: "max_results".to_string(),
                reason: format!(
                    "traversal results exceed {} (got {})",
                    MAX_TRAVERSAL_RESULTS, self.max_results
                ),
            });
        }
        Ok(())
    }
}

impl Singularity {
    /// Get direct neighbors of a concept with edge strengths.
    ///
    /// Returns outbound associations with strength >= `min_strength`.
    pub fn neighbors(&self, ns: &str, id: &str, min_strength: f32) -> Vec<(String, f32)> {
        self.get_associations(ns, id)
            .into_iter()
            .filter(|(_, strength)| *strength >= min_strength)
            .collect()
    }

    /// Get incoming associations for a concept.
    ///
    /// Returns concepts that have associations pointing to this concept.
    /// Breadth-first traversal from a starting concept.
    ///
    /// Returns nodes reachable within `config.max_depth` hops, along with their depths.
    /// Nodes are returned in BFS order.
    pub fn bfs(
        &self,
        ns: &str,
        start: &str,
        config: &TraversalConfig,
    ) -> Result<Vec<(String, u32)>> {
        config.validate()?;
        let ns_state = self
            .get_namespace(ns)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Namespace".to_string(),
                id: ns.to_string(),
            })?;
        if !ns_state.concepts.contains_key(start) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: start.to_string(),
            });
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut results: Vec<(String, u32)> = Vec::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();

        visited.insert(start.to_string());
        queue.push_back((start.to_string(), 0));

        while let Some((current, depth)) = queue.pop_front() {
            if results.len() >= config.max_results {
                break;
            }

            results.push((current.clone(), depth));

            if depth as usize >= config.max_depth {
                continue;
            }

            let neighbors = self.neighbors(ns, &current, config.min_strength);
            for (neighbor, _) in neighbors {
                if visited.insert(neighbor.clone()) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(results)
    }

    /// Find the minimum-cost path between two concepts using weighted Dijkstra.
    ///
    /// Edge cost is `-ln(strength)`, so stronger associations have lower cost.
    /// A strength of `1.0` has cost `0.0`; a strength of `0.1` has cost `~2.3`.
    /// Strength values ≤ 0 are treated as cost `f32::MAX` (effectively unreachable).
    ///
    /// Returns `None` if no path exists within `config.max_depth` hops.
    /// Use [`Self::shortest_path_hops`] for unweighted (fewest-hop) traversal.
    pub fn shortest_path(
        &self,
        ns: &str,
        from: &str,
        to: &str,
        config: &TraversalConfig,
    ) -> Result<Option<Vec<String>>> {
        config.validate()?;
        let ns_state = self
            .get_namespace(ns)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Namespace".to_string(),
                id: ns.to_string(),
            })?;
        if !ns_state.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if !ns_state.concepts.contains_key(to) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: to.to_string(),
            });
        }

        if from == to {
            return Ok(Some(vec![from.to_string()]));
        }

        // Dijkstra: min-heap of (cost_bits, depth, node_id)
        // We store cost as ordered bits via f32::to_bits for BinaryHeap<Reverse<...>>.
        let mut dist: HashMap<String, f32> = HashMap::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        // BinaryHeap is a max-heap; Reverse makes it a min-heap.
        let mut heap: BinaryHeap<Reverse<(u32, u32, String)>> = BinaryHeap::new();

        dist.insert(from.to_string(), 0.0);
        heap.push(Reverse((0u32, 0u32, from.to_string())));

        while let Some(Reverse((cost_bits, depth, current))) = heap.pop() {
            if current == to {
                // Reconstruct path
                let mut path = vec![to.to_string()];
                let mut node = to;
                while let Some(p) = parent.get(node) {
                    path.push(p.clone());
                    node = p;
                    if node == from {
                        break;
                    }
                }
                path.reverse();
                return Ok(Some(path));
            }

            let current_cost = f32::from_bits(cost_bits);
            if let Some(&best) = dist.get(&current) {
                if current_cost > best {
                    continue; // Stale entry
                }
            }

            if depth as usize >= config.max_depth {
                continue;
            }

            let neighbors = self.neighbors(ns, &current, config.min_strength);
            for (neighbor, strength) in neighbors {
                // Cost: -ln(strength), guarding against strength <= 0
                let edge_cost = if strength > 0.0 {
                    -strength.ln()
                } else {
                    f32::MAX / 2.0
                };
                let new_cost = current_cost + edge_cost;
                let best = dist.get(&neighbor).copied().unwrap_or(f32::MAX);
                if new_cost < best {
                    dist.insert(neighbor.clone(), new_cost);
                    parent.insert(neighbor.clone(), current.clone());
                    heap.push(Reverse((new_cost.to_bits(), depth + 1, neighbor)));
                }
            }
        }

        Ok(None)
    }

    /// Find the fewest-hop path between two concepts using unweighted BFS.
    ///
    /// Returns the path with the minimum number of hops, ignoring edge strengths.
    /// Use [`Self::shortest_path`] for strength-weighted (Dijkstra) traversal.
    ///
    /// Returns `None` if no path exists within `config.max_depth` hops.
    pub fn shortest_path_hops(
        &self,
        ns: &str,
        from: &str,
        to: &str,
        config: &TraversalConfig,
    ) -> Result<Option<Vec<String>>> {
        config.validate()?;
        let ns_state = self
            .get_namespace(ns)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Namespace".to_string(),
                id: ns.to_string(),
            })?;
        if !ns_state.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if !ns_state.concepts.contains_key(to) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: to.to_string(),
            });
        }

        if from == to {
            return Ok(Some(vec![from.to_string()]));
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut queue: VecDeque<(String, u32)> = VecDeque::new();

        visited.insert(from.to_string());
        queue.push_back((from.to_string(), 0));

        while let Some((current, depth)) = queue.pop_front() {
            if depth as usize >= config.max_depth {
                continue;
            }

            let neighbors = self.neighbors(ns, &current, config.min_strength);
            for (neighbor, _) in neighbors {
                if visited.insert(neighbor.clone()) {
                    parent.insert(neighbor.clone(), current.clone());
                    if neighbor == to {
                        // Reconstruct path
                        let mut path = vec![to.to_string()];
                        let mut node = to;
                        while let Some(p) = parent.get(node) {
                            path.push(p.clone());
                            node = p;
                            if node == from {
                                break;
                            }
                        }
                        path.reverse();
                        return Ok(Some(path));
                    }
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singularity::{Concept, ConceptBuilder, Singularity, SingularityConfig};
    use csm_core::hyperdim::HVec10240;

    fn make_concept(id: &str) -> Concept {
        ConceptBuilder::new(id)
            .with_vector(HVec10240::random())
            .build()
            .unwrap()
    }

    #[test]
    fn test_neighbors() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "b", 0.8).unwrap();
        sing.associate("_default", "a", "c", 0.3).unwrap();

        let neighbors = sing.neighbors("_default", "a", 0.5);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0, "b");
    }

    #[test]
    fn test_incoming_associations() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "c", 0.8).unwrap();
        sing.associate("_default", "b", "c", 0.5).unwrap();

        let incoming = sing.incoming_associations("_default", "c");
        assert_eq!(incoming.len(), 2);
        // Sorted by strength descending
        assert_eq!(incoming[0].0, "a");
        assert_eq!(incoming[1].0, "b");
    }

    #[test]
    fn test_bfs_simple() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "b", 0.5).unwrap();
        sing.associate("_default", "b", "c", 0.5).unwrap();

        let config = TraversalConfig::default();
        let results = sing.bfs("_default", "a", &config).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ("a".to_string(), 0));
        assert_eq!(results[1], ("b".to_string(), 1));
        assert_eq!(results[2], ("c".to_string(), 2));
    }

    #[test]
    fn test_bfs_max_depth() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "b", 0.5).unwrap();
        sing.associate("_default", "b", "c", 0.5).unwrap();

        let config = TraversalConfig {
            max_depth: 1,
            ..Default::default()
        };
        let results = sing.bfs("_default", "a", &config).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bfs_missing_concept() {
        let sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        let config = TraversalConfig::default();
        let result = sing.bfs("_default", "nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_shortest_path_direct() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.associate("_default", "a", "b", 0.9).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("_default", "a", "b", &config).unwrap();
        assert_eq!(path, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_shortest_path_indirect() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "b", 0.9).unwrap();
        sing.associate("_default", "b", "c", 0.9).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("_default", "a", "c", &config).unwrap();
        assert_eq!(
            path,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn test_shortest_path_no_path() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        // No association

        let config = TraversalConfig::default();
        let path = sing.shortest_path("_default", "a", "b", &config).unwrap();
        assert!(path.is_none());
    }

    #[test]
    fn test_shortest_path_same_node() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("_default", "a", "a", &config).unwrap();
        assert_eq!(path, Some(vec!["a".to_string()]));
    }

    /// Dijkstra prefers the high-strength path (lower cost = -ln(strength)).
    #[test]
    fn test_shortest_path_dijkstra_prefers_strong_edge() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        // a --0.9--> b --0.9--> d  (strong path, 2 hops)
        // a --0.1--> c --0.1--> d  (weak path, 2 hops)
        for id in ["a", "b", "c", "d"] {
            sing.inject("_default", make_concept(id)).unwrap();
        }
        sing.associate("_default", "a", "b", 0.9).unwrap();
        sing.associate("_default", "b", "d", 0.9).unwrap();
        sing.associate("_default", "a", "c", 0.1).unwrap();
        sing.associate("_default", "c", "d", 0.1).unwrap();

        let config = TraversalConfig::default();
        let path = sing
            .shortest_path("_default", "a", "d", &config)
            .unwrap()
            .unwrap();
        // Strong path a→b→d has lower cost than weak path a→c→d
        assert_eq!(path, vec!["a", "b", "d"]);
    }

    #[test]
    fn test_shortest_path_hops_direct() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.associate("_default", "a", "b", 0.5).unwrap();

        let config = TraversalConfig::default();
        let path = sing
            .shortest_path_hops("_default", "a", "b", &config)
            .unwrap();
        assert_eq!(path, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_shortest_path_hops_indirect() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();
        sing.inject("_default", make_concept("c")).unwrap();
        sing.associate("_default", "a", "b", 0.5).unwrap();
        sing.associate("_default", "b", "c", 0.5).unwrap();

        let config = TraversalConfig::default();
        let path = sing
            .shortest_path_hops("_default", "a", "c", &config)
            .unwrap();
        assert_eq!(
            path,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn test_shortest_path_hops_no_path() {
        let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
        sing.inject("_default", make_concept("a")).unwrap();
        sing.inject("_default", make_concept("b")).unwrap();

        let config = TraversalConfig::default();
        let path = sing
            .shortest_path_hops("_default", "a", "b", &config)
            .unwrap();
        assert!(path.is_none());
    }
}
