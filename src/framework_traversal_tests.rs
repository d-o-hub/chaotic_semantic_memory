#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Unit-target tests for the graph-traversal facade (`src/framework_traversal.rs`).
//!
//! `traverse` and `shortest_path` are two-line wrappers over `Singularity::bfs`
//! and `Singularity::shortest_path`, and every guard they own — ID validation,
//! config validation, and the fact that they return the traversal at all — was
//! mutation-invisible: replacing either whole body with an empty `Ok(..)` left the
//! suite green.

use crate::framework::ChaoticSemanticFramework;
use crate::graph_traversal::TraversalConfig;
use csm_core_lib::hyperdim::HVec10240;

async fn chain_framework() -> ChaoticSemanticFramework {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    for id in ["a", "b", "c", "d"] {
        framework
            .inject_concept(id, HVec10240::random())
            .await
            .unwrap();
    }
    // a -> b -> d is the two-hop spine; a -> c is a weak side branch.
    framework.associate("a", "b", 0.9).await.unwrap();
    framework.associate("b", "d", 0.8).await.unwrap();
    framework.associate("a", "c", 0.2).await.unwrap();
    framework
}

#[tokio::test]
async fn traverse_reports_each_visited_concept_with_its_hop_count() {
    let framework = chain_framework().await;
    let mut visited = framework
        .traverse("a", TraversalConfig::default())
        .await
        .unwrap();
    visited.sort_unstable();

    assert_eq!(
        visited,
        vec![
            ("a".to_string(), 0),
            ("b".to_string(), 1),
            ("c".to_string(), 1),
            ("d".to_string(), 2),
        ],
        "breadth-first traversal must label each concept by its distance from the start"
    );
}

#[tokio::test]
async fn traverse_honours_max_depth_min_strength_and_max_results() {
    let framework = chain_framework().await;

    let shallow = framework
        .traverse(
            "a",
            TraversalConfig {
                max_depth: 1,
                ..TraversalConfig::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !shallow.iter().any(|(id, _)| id == "d"),
        "depth 1 must not expand past the first hop: {shallow:?}"
    );

    let strict = framework
        .traverse(
            "a",
            TraversalConfig {
                min_strength: 0.5,
                ..TraversalConfig::default()
            },
        )
        .await
        .unwrap();
    assert!(
        !strict.iter().any(|(id, _)| id == "c"),
        "an edge below min_strength must not be followed: {strict:?}"
    );

    let capped = framework
        .traverse(
            "a",
            TraversalConfig {
                max_results: 1,
                ..TraversalConfig::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(capped, vec![("a".to_string(), 0)]);
}

#[tokio::test]
async fn traverse_rejects_an_unknown_start_and_an_invalid_request() {
    let framework = chain_framework().await;

    assert!(
        framework
            .traverse("missing", TraversalConfig::default())
            .await
            .is_err(),
        "traversing from a concept that was never injected must not yield an empty walk"
    );
    assert!(
        framework
            .traverse("", TraversalConfig::default())
            .await
            .is_err(),
        "the facade owns concept-ID validation"
    );
    assert!(
        framework
            .traverse(
                "a",
                TraversalConfig {
                    max_depth: 33, // framework_validation.rs:13 MAX_TRAVERSAL_DEPTH = 32
                    ..TraversalConfig::default()
                },
            )
            .await
            .is_err(),
        "the facade owns traversal-config validation"
    );
    assert!(
        framework
            .traverse(
                "a",
                TraversalConfig {
                    max_results: 10_001, // framework_validation.rs:14 MAX_TRAVERSAL_RESULTS
                    ..TraversalConfig::default()
                },
            )
            .await
            .is_err(),
        "an over-large result cap must be rejected before the graph is walked"
    );
}

#[tokio::test]
async fn shortest_path_returns_the_hops_in_order() {
    let framework = chain_framework().await;
    assert_eq!(
        framework.shortest_path("a", "d").await.unwrap(),
        Some(vec!["a".to_string(), "b".to_string(), "d".to_string()]),
        "the path must include both endpoints, ordered from the start"
    );
    assert_eq!(
        framework.shortest_path("a", "a").await.unwrap(),
        Some(vec!["a".to_string()]),
        "a concept is one hop from itself"
    );
}

#[tokio::test]
async fn shortest_path_distinguishes_unreachable_from_invalid() {
    let framework = chain_framework().await;
    assert_eq!(
        framework.shortest_path("d", "a").await.unwrap(),
        None,
        "edges are directional, so the reverse walk has no path"
    );
    assert!(
        framework.shortest_path("a", "").await.is_err(),
        "an empty target ID must be rejected, not reported as no-path"
    );
    assert!(
        framework.shortest_path("a", "ghost").await.is_err(),
        "a concept that was never injected must be a lookup error, not None"
    );
}
