#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::singularity::{Concept, ConceptBuilder, Singularity, SingularityConfig};
use csm_core_lib::hyperdim::HVec10240;

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
