#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Reconciliation between memory and durable rows, in the `--lib` target.
//!
//! The same two behaviours are pinned by `tests/framework_lifecycle.rs`, but the
//! mutation gate runs `cargo test --lib` only, so an integration test cannot kill a
//! mutant here however precisely it pins the observable.

use crate::concept_builder::ConceptBuilder;
use crate::framework::ChaoticSemanticFramework;
use csm_core_lib::hyperdim::HVec10240;
use tempfile::tempdir;

#[tokio::test]
async fn load_merge_pulls_in_a_concept_another_writer_persisted() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("merge.db");
    let db_a = db.to_str().unwrap().to_string();
    let db_b = db.to_str().unwrap().to_string();

    let framework_a = ChaoticSemanticFramework::builder()
        .with_local_db(&db_a)
        .build()
        .await
        .expect("framework A");
    framework_a
        .inject_concept("a-owned", HVec10240::random())
        .await
        .expect("inject A");

    let framework_b = ChaoticSemanticFramework::builder()
        .with_local_db(&db_b)
        .build()
        .await
        .expect("framework B");
    framework_b
        .inject_concept("b-owned", HVec10240::random())
        .await
        .expect("inject B");

    assert!(
        framework_a
            .get_concept("b-owned")
            .await
            .expect("get_concept")
            .is_none(),
        "A must not see B's write before it asks for one"
    );

    framework_a.load_merge().await.expect("load_merge");

    assert!(
        framework_a
            .get_concept("b-owned")
            .await
            .expect("get_concept")
            .is_some(),
        "load_merge must bring the durably written concept into memory"
    );
    assert!(
        framework_a
            .get_concept("a-owned")
            .await
            .expect("get_concept")
            .is_some(),
        "merging must not drop what A already held"
    );
}

#[tokio::test]
async fn reload_namespace_from_rows_discards_memory_that_was_never_persisted() {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("reload.db");
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(db.to_str().unwrap())
        .build()
        .await
        .expect("framework");

    framework
        .inject_concept("durable", HVec10240::random())
        .await
        .expect("inject durable");

    // Memory-only write: straight into the singularity, bypassing persistence.
    let ghost = ConceptBuilder::new("memory-only")
        .with_vector(HVec10240::random())
        .build()
        .expect("concept");
    framework
        .singularity
        .write()
        .await
        .inject("_default", ghost)
        .expect("memory inject");
    assert!(
        framework
            .get_concept("memory-only")
            .await
            .expect("get_concept")
            .is_some(),
        "the unpersisted concept must be in memory before the reload"
    );

    framework
        .reload_namespace_from_rows("_default")
        .await
        .expect("reload_namespace_from_rows");

    assert!(
        framework
            .get_concept("memory-only")
            .await
            .expect("get_concept")
            .is_none(),
        "a reload from authoritative rows must drop unpersisted memory"
    );
    assert!(
        framework
            .get_concept("durable")
            .await
            .expect("get_concept")
            .is_some(),
        "the reload must restore what the rows actually hold"
    );
}
