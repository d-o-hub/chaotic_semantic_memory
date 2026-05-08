use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240, Hypervector};

const NS: &str = "_default";

#[tokio::test]
async fn concept_id_enforces_256_byte_limit() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let id_256 = "a".repeat(256);
    framework
        .inject_concept(id_256, HVec10240::random())
        .await
        .unwrap();

    let id_257 = "b".repeat(257);
    let err = framework
        .inject_concept(id_257, HVec10240::random())
        .await
        .unwrap_err();
    let text = err.to_string();
    assert!(text.contains("exceeds 256 bytes"));
}

#[tokio::test]
async fn associate_rejects_negative_strength() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("a", HVec10240::random())
        .await
        .unwrap();
    framework
        .inject_concept("b", HVec10240::random())
        .await
        .unwrap();

    let err = framework.associate("a", "b", -0.1).await.unwrap_err();
    let text = err.to_string();
    assert!(text.contains("must be in [0.0, 1.0]"));
}

#[test]
fn reservoir_to_hypervector_dimension_boundary() {
    let too_small =
        chaotic_semantic_memory::reservoir::Reservoir::new_seeded(64, 10_239, 7).unwrap();
    assert!(too_small.to_hypervector().is_err());

    let valid = chaotic_semantic_memory::reservoir::Reservoir::new_seeded(64, 10_240, 7).unwrap();
    assert!(valid.to_hypervector().is_ok());
}

#[tokio::test]
async fn probe_respects_configured_top_k_limit() {
    let framework: ChaoticSemanticFramework<HVec10240> = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_max_probe_top_k(3)
        .build()
        .await
        .unwrap();

    framework
        .inject_concept("topic", HVec10240::random())
        .await
        .unwrap();

    let err = framework.probe(&HVec10240::random(), 4).await.unwrap_err();
    let text = err.to_string();
    assert!(text.contains("configured limit"));
}
