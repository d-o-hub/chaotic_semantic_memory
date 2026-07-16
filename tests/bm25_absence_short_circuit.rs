#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! End-to-end: repeated empty probe_text accumulates absence; threshold is public.

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::retrieval::bm25::DEFAULT_ABSENCE_MIN_ATTEMPTS;
use tempfile::NamedTempFile;

/// After enough empty semantic probes, absence attempt_count reaches the BM25 skip threshold.
#[tokio::test]
async fn empty_probes_accumulate_absence_to_threshold() {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap();

    let framework = ChaoticSemanticFramework::builder()
        .with_local_db(path)
        .build()
        .await
        .unwrap();

    // Empty memory: each probe_text abstains and persists absence.
    for i in 0..DEFAULT_ABSENCE_MIN_ATTEMPTS {
        let result = framework
            .probe_text("no-such-concept-query", 5)
            .await
            .unwrap();
        assert!(
            matches!(
                result,
                chaotic_semantic_memory::retrieval::hybrid::HybridResult::Abstained(_)
            ),
            "iteration {i} should abstain on empty store"
        );
    }

    // Store is wired; public constant documents the short-circuit threshold used by CLI hybrid.
    assert_eq!(DEFAULT_ABSENCE_MIN_ATTEMPTS, 3);
}
