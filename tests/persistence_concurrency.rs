#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "persistence")]
//! Concurrent local writes must not surface lock errors (ADR-0095).
//!
//! The 2026-09-17 scale evidence measured a 90 % error rate with eight
//! concurrent writers on one local database before `csm-persistence` set a
//! busy timeout and bounded retries
//! (`plans/evidence/scale_2026_09_17/persistence_scale.json`). This test is
//! the regression guard: every write must succeed and every row must be
//! readable afterwards.

use std::sync::Arc;

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::singularity::Concept;

const NS: &str = "_default";
const TASKS: usize = 8;
const OPS: usize = 25;

fn concept(task: usize, op: usize) -> Concept {
    Concept {
        id: format!("t{task}-op{op}"),
        vector: HVec10240::zero(),
        metadata: Default::default(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_concept_writes_all_succeed() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("concurrency.db");
    let persistence = Arc::new(
        Persistence::new_local(db.to_str().unwrap())
            .await
            .expect("open local database"),
    );

    let mut handles = Vec::with_capacity(TASKS);
    for task in 0..TASKS {
        let store = Arc::clone(&persistence);
        handles.push(tokio::spawn(async move {
            for op in 0..OPS {
                store
                    .save_concept(NS, &concept(task, op))
                    .await
                    .expect("concurrent save must not surface a lock error");
            }
        }));
    }
    for handle in handles {
        handle.await.expect("writer task");
    }

    let loaded = persistence
        .load_all_concepts::<HVec10240>(NS)
        .await
        .expect("load all concepts");
    assert_eq!(
        loaded.len(),
        TASKS * OPS,
        "every concurrently written concept must be durable"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_batch_and_association_writes_all_succeed() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = dir.path().join("concurrency_batch.db");
    let persistence = Arc::new(
        Persistence::new_local(db.to_str().unwrap())
            .await
            .expect("open local database"),
    );

    // The endpoints must exist for the association foreign keys.
    for task in 0..TASKS {
        persistence
            .save_concept(NS, &concept(task, 0))
            .await
            .expect("seed concept");
    }

    let mut handles = Vec::with_capacity(TASKS);
    for task in 0..TASKS {
        let store = Arc::clone(&persistence);
        handles.push(tokio::spawn(async move {
            let batch: Vec<Concept> = (1..=5).map(|op| concept(task, op)).collect();
            store
                .save_concepts(NS, &batch)
                .await
                .expect("concurrent batch save must not surface a lock error");
            for op in 1..=5 {
                store
                    .save_association(NS, &format!("t{task}-op{op}"), &format!("t{task}-op0"), 0.5)
                    .await
                    .expect("concurrent association save must not surface a lock error");
            }
        }));
    }
    for handle in handles {
        handle.await.expect("writer task");
    }

    let loaded = persistence
        .load_all_concepts::<HVec10240>(NS)
        .await
        .expect("load all concepts");
    assert_eq!(loaded.len(), TASKS * 6);
}
