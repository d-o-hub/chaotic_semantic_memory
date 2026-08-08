//! Wave 32: single-query association load (`load_all_associations`).
//!
//! Verifies completeness for large namespaces (multi-target, duplicate upserts),
//! namespace isolation, empty namespaces, and that a framework load no longer
//! deadlocks or starves when racing inject/probe/persist.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::Concept;
use tempfile::NamedTempFile;

const NS: &str = "_default";

fn make_concept(id: &str, created_at: u64, modified_at: u64) -> Concept {
    Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at,
        modified_at,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

#[tokio::test]
async fn load_all_associations_returns_every_row_for_large_namespace() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let persistence = Persistence::new_local(&path).await.unwrap();

    let concepts: Vec<Concept> = (0..120)
        .map(|i| make_concept(&format!("c-{i:03}"), i as u64, i as u64))
        .collect();
    persistence.save_concepts(NS, &concepts).await.unwrap();

    for i in 0..120 {
        let from = format!("c-{i:03}");
        let first = format!("c-{:03}", (i + 1) % 120);
        let second = format!("c-{:03}", (i + 2) % 120);
        persistence
            .save_association(NS, &from, &first, 0.5)
            .await
            .unwrap();
        // Duplicate (from, to): upsert collapses to one row with new strength.
        persistence
            .save_association(NS, &from, &first, 0.9)
            .await
            .unwrap();
        persistence
            .save_association(NS, &from, &second, 0.7)
            .await
            .unwrap();
    }

    let all = persistence.load_all_associations(NS).await.unwrap();
    assert_eq!(all.len(), 240);

    // Every row the per-concept loader sees must appear exactly once here.
    for i in 0..120 {
        let from = format!("c-{i:03}");
        let per_concept = persistence.load_associations(NS, &from).await.unwrap();
        assert_eq!(per_concept.len(), 2);

        let via_all: Vec<_> = all.iter().filter(|(f, _, _, _)| f == &from).collect();
        assert_eq!(via_all.len(), 2);

        let mut strengths: Vec<f32> = via_all.iter().map(|(_, _, s, _)| *s).collect();
        strengths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(strengths, vec![0.7, 0.9]);
    }

    // Deterministic ordering: ascending by (from_id, to_id).
    for window in all.windows(2) {
        assert!(window[0].0 <= window[1].0);
        if window[0].0 == window[1].0 {
            assert!(window[0].1 <= window[1].1);
        }
    }
}

#[tokio::test]
async fn load_all_associations_is_scoped_to_namespace() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let persistence = Persistence::new_local(&path).await.unwrap();

    persistence
        .save_concepts(NS, &[make_concept("a", 1, 1), make_concept("b", 1, 1)])
        .await
        .unwrap();
    persistence.save_association(NS, "a", "b", 0.6).await.unwrap();

    let other = "other_ns";
    persistence
        .save_concepts(other, &[make_concept("x", 1, 1), make_concept("y", 1, 1)])
        .await
        .unwrap();
    persistence.save_association(other, "x", "y", 0.4).await.unwrap();

    let in_default = persistence.load_all_associations(NS).await.unwrap();
    assert_eq!(in_default.len(), 1);
    assert_eq!(in_default[0].0, "a");
    assert_eq!(in_default[0].1, "b");

    let in_other = persistence.load_all_associations(other).await.unwrap();
    assert_eq!(in_other.len(), 1);
    assert_eq!(in_other[0].0, "x");
}

#[tokio::test]
async fn load_all_associations_empty_for_unknown_namespace() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();
    let persistence = Persistence::new_local(&path).await.unwrap();

    assert!(persistence
        .load_all_associations("no-such-namespace")
        .await
        .unwrap()
        .is_empty());

    // An existing namespace with a concept but no associations is also empty.
    persistence
        .save_concepts(NS, &[make_concept("solo", 1, 1)])
        .await
        .unwrap();
    assert!(persistence.load_all_associations(NS).await.unwrap().is_empty());
}

#[tokio::test]
async fn load_replace_restores_all_associations_for_large_namespace() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .with_max_concepts(1000)
        .with_max_associations_per_concept(16)
        .build()
        .await
        .unwrap();

    for i in 0..120 {
        framework
            .inject_concept(format!("n-{i:03}"), HVec10240::random())
            .await
            .unwrap();
    }
    for i in 0..120 {
        let from = format!("n-{i:03}");
        framework
            .associate(&from, &format!("n-{:03}", (i + 1) % 120), 0.5)
            .await
            .unwrap();
        framework
            .associate(&from, &format!("n-{:03}", (i + 2) % 120), 0.7)
            .await
            .unwrap();
    }
    framework.persist().await.unwrap();

    let reloaded = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();
    reloaded.load_replace().await.unwrap();

    assert_eq!(reloaded.stats().await.unwrap().concept_count, 120);
    for i in 0..120 {
        let links = reloaded
            .get_associations(&format!("n-{i:03}"))
            .await
            .unwrap();
        assert_eq!(links.len(), 2);
    }
}

#[tokio::test]
async fn concurrent_inject_probe_persist_load_does_not_deadlock() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_string();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path.clone())
        .with_max_concepts(1000)
        .with_max_associations_per_concept(64)
        .build()
        .await
        .unwrap();

    // Seed persisted content so a concurrent load has rows to fetch.
    for i in 0..10 {
        framework
            .inject_concept(format!("seed-{i}"), HVec10240::random())
            .await
            .unwrap();
        framework
            .associate("seed-0", &format!("seed-{i}"), 0.5)
            .await
            .unwrap();
    }

    let inject = |i: usize| {
        let framework = &framework;
        async move {
            let id = format!("live-{i}");
            framework
                .inject_concept(id, HVec10240::random())
                .await
                .unwrap();
            // A racing load_replace may clear a freshly injected concept out of
            // memory (NotFound here is benign for the deadlock exercise).
            let _ = framework
                .associate("seed-0", &format!("live-{i}"), 0.3)
                .await;
        }
    };
    let probe = async {
        for _ in 0..20 {
            framework.probe(HVec10240::random(), 5).await.unwrap();
        }
    };
    let persist = async {
        framework.persist().await.unwrap();
    };
    let load = async {
        framework.load_replace().await.unwrap();
    };

    let (i0, i1, i2, i3, i4, i5, i6, i7, pr, pe, lo) = tokio::join!(
        inject(0), inject(1), inject(2), inject(3),
        inject(4), inject(5), inject(6), inject(7),
        probe, persist, load,
    );
    // No deadlock: every branch returned. Results are racy by design.
    let _ = (i0, i1, i2, i3, i4, i5, i6, i7, pr, pe, lo);

    let stats = framework.stats().await.unwrap();
    assert!(stats.concept_count >= 10);
    assert!(!framework.get_associations("seed-0").await.unwrap().is_empty());
}