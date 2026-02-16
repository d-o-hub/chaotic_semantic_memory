use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240, TursoClient};

const TEST_RESERVOIR_SIZE: usize = 1024;
const TEST_TOP_K: usize = 2;
const TEST_SEED_ALPHA: u64 = 11;
const TEST_SEED_BETA: u64 = 17;

#[tokio::test]
async fn singularity_inject_and_probe_returns_ranked_results() {
    let framework = ChaoticSemanticFramework::singularity()
        .with_reservoir_size(TEST_RESERVOIR_SIZE)
        .build()
        .await
        .expect("framework build should succeed");

    let novelty_alpha = framework.inject_concept("alpha", HVec10240::from_seed(TEST_SEED_ALPHA));
    let novelty_beta = framework.inject_concept("beta", HVec10240::from_seed(TEST_SEED_BETA));

    assert!(novelty_alpha >= 0.0);
    assert!(novelty_beta >= 0.0);

    let top = framework.singularity_probe(HVec10240::from_seed(TEST_SEED_ALPHA), TEST_TOP_K);
    assert!(!top.is_empty());
    assert_eq!(top[0].0, "alpha");
}

#[tokio::test]
async fn persist_and_restore_roundtrip_works_with_file_url() {
    let db_file = std::env::temp_dir().join("chaotic_semantic_memory_test.db");
    if db_file.exists() {
        let _ = std::fs::remove_file(&db_file);
    }
    let db_url = format!("file:{}", db_file.display());
    let client =
        TursoClient::new(db_url.clone(), "test-token".to_string()).expect("client should build");

    let framework = ChaoticSemanticFramework::singularity()
        .with_turso(db_url, "test-token")
        .with_reservoir_size(TEST_RESERVOIR_SIZE)
        .build()
        .await
        .expect("framework build should succeed");

    let _ = framework.inject_concept("alpha", HVec10240::from_seed(TEST_SEED_ALPHA));
    framework
        .persist_turso(&client)
        .await
        .expect("persist should succeed");

    let restored = ChaoticSemanticFramework::restore_turso(&client)
        .await
        .expect("restore should succeed");

    let top = restored.singularity_probe(HVec10240::from_seed(TEST_SEED_ALPHA), TEST_TOP_K);
    assert!(!top.is_empty());
    assert_eq!(top[0].0, "alpha");
}

#[tokio::test]
async fn builder_rejects_zero_sync_lock_retries() {
    let result = ChaoticSemanticFramework::singularity()
        .with_sync_lock_retries(0)
        .build()
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn probe_merges_fallback_entries() {
    let framework = ChaoticSemanticFramework::singularity()
        .with_reservoir_size(TEST_RESERVOIR_SIZE)
        .build()
        .await
        .expect("framework build should succeed");

    let _ = framework.inject_concept("alpha", HVec10240::from_seed(TEST_SEED_ALPHA));
    let top = framework.singularity_probe(HVec10240::from_seed(TEST_SEED_ALPHA), TEST_TOP_K);
    assert!(top.iter().any(|(name, _)| name == "alpha"));
}
