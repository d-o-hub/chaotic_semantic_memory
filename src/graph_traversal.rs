//! Graph traversal operations on the association graph.
//!
//! Provides BFS, shortest path, and neighbor queries on the concept association graph.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::{MemoryError, Result};
use crate::singularity::Singularity;

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

impl Singularity {
    /// Get direct neighbors of a concept with edge strengths.
    ///
    /// Returns outbound associations with strength >= `min_strength`.
    pub fn neighbors(&self, id: &str, min_strength: f32) -> Vec<(String, f32)> {
        self.get_associations(id)
            .into_iter()
            .filter(|(_, strength)| *strength >= min_strength)
            .collect()
    }

    /// Get incoming associations for a concept.
    ///
    /// Returns concepts that have associations pointing to this concept.
    pub fn incoming_associations(&self, id: &str) -> Vec<(String, f32)> {
        let mut incoming = Vec::new();
        for (from_id, links) in &self.associations {
            if let Some(&strength) = links.get(id) {
                incoming.push((from_id.clone(), strength));
            }
        }
        incoming.sort_by(|a, b| b.1.total_cmp(&a.1));
        incoming
    }

    /// Breadth-first traversal from a starting concept.
    ///
    /// Returns nodes reachable within `config.max_depth` hops, along with their depths.
    /// Nodes are returned in BFS order.
    pub fn bfs(&self, start: &str, config: &TraversalConfig) -> Result<Vec<(String, u32)>> {
        if !self.concepts.contains_key(start) {
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

            let neighbors = self.neighbors(&current, config.min_strength);
            for (neighbor, _) in neighbors {
                if visited.insert(neighbor.clone()) {
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(results)
    }

    /// Find the shortest path between two concepts.
    ///
    /// Uses BFS to find the shortest path. Returns `None` if no path exists.
    /// Edge costs are computed as `-ln(strength)` to prefer stronger associations.
    pub fn shortest_path(
        &self,
        from: &str,
        to: &str,
        config: &TraversalConfig,
    ) -> Result<Option<Vec<String>>> {
        if !self.concepts.contains_key(from) {
            return Err(MemoryError::NotFound {
                entity: "Concept".to_string(),
                id: from.to_string(),
            });
        }
        if !self.concepts.contains_key(to) {
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
        let mut queue: VecDeque<String> = VecDeque::new();

        visited.insert(from.to_string());
        queue.push_back(from.to_string());

        let mut found = false;
        while let Some(current) = queue.pop_front() {
            if found {
                break;
            }

            let neighbors = self.neighbors(&current, config.min_strength);
            for (neighbor, _) in neighbors {
                if visited.insert(neighbor.clone()) {
                    parent.insert(neighbor.clone(), current.clone());
                    if neighbor == to {
                        found = true;
                        break;
                    }
                    queue.push_back(neighbor);
                }
            }
        }

        if !found {
            return Ok(None);
        }

        // Reconstruct path
        let mut path = vec![to.to_string()];
        let mut current = to;
        while let Some(p) = parent.get(current) {
            path.push(p.clone());
            current = p;
            if current == from {
                break;
            }
        }
        path.reverse();

        Ok(Some(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;
    use crate::singularity::{Concept, ConceptBuilder, Singularity};

    fn make_concept(id: &str) -> Concept {
        ConceptBuilder::new(id)
            .with_vector(HVec10240::random())
            .build()
            .unwrap()
    }

    #[test]
    fn test_neighbors() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.inject(make_concept("c")).unwrap();
        sing.associate("a", "b", 0.8).unwrap();
        sing.associate("a", "c", 0.3).unwrap();

        let neighbors = sing.neighbors("a", 0.5);
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0, "b");
    }

    #[test]
    fn test_incoming_associations() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.inject(make_concept("c")).unwrap();
        sing.associate("a", "c", 0.8).unwrap();
        sing.associate("b", "c", 0.5).unwrap();

        let incoming = sing.incoming_associations("c");
        assert_eq!(incoming.len(), 2);
        // Sorted by strength descending
        assert_eq!(incoming[0].0, "a");
        assert_eq!(incoming[1].0, "b");
    }

    #[test]
    fn test_bfs_simple() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.inject(make_concept("c")).unwrap();
        sing.associate("a", "b", 0.5).unwrap();
        sing.associate("b", "c", 0.5).unwrap();

        let config = TraversalConfig::default();
        let results = sing.bfs("a", &config).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ("a".to_string(), 0));
        assert_eq!(results[1], ("b".to_string(), 1));
        assert_eq!(results[2], ("c".to_string(), 2));
    }

    #[test]
    fn test_bfs_max_depth() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.inject(make_concept("c")).unwrap();
        sing.associate("a", "b", 0.5).unwrap();
        sing.associate("b", "c", 0.5).unwrap();

        let config = TraversalConfig {
            max_depth: 1,
            ..Default::default()
        };
        let results = sing.bfs("a", &config).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bfs_missing_concept() {
        let sing = Singularity::new();
        let config = TraversalConfig::default();
        let result = sing.bfs("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_shortest_path_direct() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.associate("a", "b", 0.5).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("a", "b", &config).unwrap();
        assert_eq!(path, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_shortest_path_indirect() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        sing.inject(make_concept("c")).unwrap();
        sing.associate("a", "b", 0.5).unwrap();
        sing.associate("b", "c", 0.5).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("a", "c", &config).unwrap();
        assert_eq!(
            path,
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn test_shortest_path_no_path() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();
        sing.inject(make_concept("b")).unwrap();
        // No association

        let config = TraversalConfig::default();
        let path = sing.shortest_path("a", "b", &config).unwrap();
        assert!(path.is_none());
    }

    #[test]
    fn test_shortest_path_same_node() {
        let mut sing = Singularity::new();
        sing.inject(make_concept("a")).unwrap();

        let config = TraversalConfig::default();
        let path = sing.shortest_path("a", "a", &config).unwrap();
        assert_eq!(path, Some(vec!["a".to_string()]));
    }
}
