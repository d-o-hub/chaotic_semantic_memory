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
use csm_core_lib::error::{MemoryError, Result};

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
        let (start_key, _) = ns_state
            .concepts
            .get_key_value(start)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: start.to_string(),
            })?;
        let start_str = start_key.as_str();

        // Memory Optimization: Use borrowed &str references to eliminate transient String allocations
        // and repeated namespace lock re-acquisitions during graph traversal.
        let mut visited: HashSet<&str> = HashSet::new();
        let mut results: Vec<(String, u32)> = Vec::new();
        let mut queue: VecDeque<(&str, u32)> = VecDeque::new();

        visited.insert(start_str);
        queue.push_back((start_str, 0));

        while let Some((current, depth)) = queue.pop_front() {
            if results.len() >= config.max_results {
                break;
            }

            results.push((current.to_string(), depth));

            if depth as usize >= config.max_depth {
                continue;
            }

            if let Some(neighbors) = ns_state.associations.get(current) {
                for (neighbor_key, &(strength, _)) in neighbors {
                    if strength >= config.min_strength {
                        let neighbor_str = neighbor_key.as_str();
                        if visited.insert(neighbor_str) {
                            queue.push_back((neighbor_str, depth + 1));
                        }
                    }
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
        let (from_key, _) = ns_state
            .concepts
            .get_key_value(from)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            })?;
        let (to_key, _) = ns_state
            .concepts
            .get_key_value(to)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: to.to_string(),
            })?;
        let from_str = from_key.as_str();
        let to_str = to_key.as_str();

        if from == to {
            return Ok(Some(vec![from.to_string()]));
        }

        // Memory Optimization: Use borrowed &str references to eliminate transient String allocations
        // and repeated namespace lock re-acquisitions during Dijkstra path calculation.
        // Dijkstra: min-heap of (cost_bits, depth, node_id)
        let mut dist: HashMap<&str, f32> = HashMap::new();
        let mut parent: HashMap<&str, &str> = HashMap::new();
        // BinaryHeap is a max-heap; Reverse makes it a min-heap.
        let mut heap: BinaryHeap<Reverse<(u32, u32, &str)>> = BinaryHeap::new();

        dist.insert(from_str, 0.0);
        heap.push(Reverse((0u32, 0u32, from_str)));

        while let Some(Reverse((cost_bits, depth, current))) = heap.pop() {
            if current == to_str {
                // Reconstruct path
                let mut path = vec![to_str.to_string()];
                let mut node = to_str;
                while let Some(&p) = parent.get(node) {
                    path.push(p.to_string());
                    node = p;
                    if node == from_str {
                        break;
                    }
                }
                path.reverse();
                return Ok(Some(path));
            }

            let current_cost = f32::from_bits(cost_bits);
            if let Some(&best) = dist.get(current) {
                if current_cost > best {
                    continue; // Stale entry
                }
            }

            if depth as usize >= config.max_depth {
                continue;
            }

            if let Some(neighbors) = ns_state.associations.get(current) {
                for (neighbor_key, &(strength, _)) in neighbors {
                    if strength >= config.min_strength {
                        // Cost: -ln(strength), guarding against strength <= 0
                        let edge_cost = if strength > 0.0 {
                            -strength.ln()
                        } else {
                            f32::MAX / 2.0
                        };
                        let new_cost = current_cost + edge_cost;
                        let neighbor_str = neighbor_key.as_str();
                        let best = dist.get(neighbor_str).copied().unwrap_or(f32::MAX);
                        if new_cost < best {
                            dist.insert(neighbor_str, new_cost);
                            parent.insert(neighbor_str, current);
                            heap.push(Reverse((new_cost.to_bits(), depth + 1, neighbor_str)));
                        }
                    }
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
        let (from_key, _) = ns_state
            .concepts
            .get_key_value(from)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            })?;
        let (to_key, _) = ns_state
            .concepts
            .get_key_value(to)
            .ok_or_else(|| MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: to.to_string(),
            })?;
        let from_str = from_key.as_str();
        let to_str = to_key.as_str();

        if from == to {
            return Ok(Some(vec![from.to_string()]));
        }

        // Memory Optimization: Use borrowed &str references to eliminate transient String allocations
        // and repeated namespace lock re-acquisitions during unweighted BFS traversal.
        let mut visited: HashSet<&str> = HashSet::new();
        let mut parent: HashMap<&str, &str> = HashMap::new();
        let mut queue: VecDeque<(&str, u32)> = VecDeque::new();

        visited.insert(from_str);
        queue.push_back((from_str, 0));

        while let Some((current, depth)) = queue.pop_front() {
            if depth as usize >= config.max_depth {
                continue;
            }

            if let Some(neighbors) = ns_state.associations.get(current) {
                for (neighbor_key, &(strength, _)) in neighbors {
                    if strength >= config.min_strength {
                        let neighbor_str = neighbor_key.as_str();
                        if visited.insert(neighbor_str) {
                            parent.insert(neighbor_str, current);
                            if neighbor_str == to_str {
                                // Reconstruct path
                                let mut path = vec![to_str.to_string()];
                                let mut node = to_str;
                                while let Some(&p) = parent.get(node) {
                                    path.push(p.to_string());
                                    node = p;
                                    if node == from_str {
                                        break;
                                    }
                                }
                                path.reverse();
                                return Ok(Some(path));
                            }
                            queue.push_back((neighbor_str, depth + 1));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
#[path = "graph_traversal_tests.rs"]
mod tests;
