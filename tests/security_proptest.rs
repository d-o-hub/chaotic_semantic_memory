use chaotic_semantic_memory::hyperdim::HVec10240;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::reservoir::Reservoir;
use proptest::prelude::*;
use std::collections::HashMap;

// Helper to establish proper macro context
fn _unused() {}

proptest! {
    #[test]
    fn concept_id_too_long_rejected_via_api(id in "[a]{257,300}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_err());
        if let Err(MemoryError::InvalidInput { field, reason }) = result {
            prop_assert_eq!(field, "id");
            prop_assert!(reason.contains("exceeds"));
        }
    }

    #[test]
    fn concept_id_at_max_length_accepted_via_api(id in "[a-z0-9_-]{256}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_ok());
    }

    #[test]
    fn concept_id_shorter_than_max_accepted_via_api(id in "[a-z0-9_-]{1,255}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_ok());
    }

    #[test]
    fn concept_id_control_chars_rejected_via_api(
        prefix in "[a-z]+",
        control in any::<char>().prop_filter("is_control", |c| c.is_control()),
        suffix in "[a-z]+"
    ) {
        let id = format!("{}{}{}", prefix, control, suffix);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_err());
    }

    #[test]
    fn association_strength_invalid_rejected_via_api(
        strength in prop_oneof![Just(f32::NAN), Just(f32::INFINITY), Just(f32::NEG_INFINITY)]
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept("a", HVec10240::random()).await.unwrap();
            framework.inject_concept("b", HVec10240::random()).await.unwrap();
            framework.associate("a", "b", strength).await
        });
        prop_assert!(result.is_err());
    }

    #[test]
    fn association_strength_valid_accepted_via_api(strength in 0.0f32..=1.0f32) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let framework = ChaoticSemanticFramework::builder()
                .without_persistence()
                .build()
                .await
                .unwrap();
            framework.inject_concept("a", HVec10240::random()).await.unwrap();
            framework.inject_concept("b", HVec10240::random()).await.unwrap();
            framework.associate("a", "b", strength).await
        });
        prop_assert!(result.is_ok());
    }

    #[test]
    fn max_concepts_boundary_enforced(max_concepts in 1usize..10) {
        let mut singularity = chaotic_semantic_memory::singularity::Singularity::with_config(
            chaotic_semantic_memory::singularity::SingularityConfig {
                max_concepts: Some(max_concepts),
                max_associations_per_concept: None,
                concept_cache_size: 64,
                max_cached_top_k: 100,
            }
        );
        for i in 0..(max_concepts + 5) {
            let concept = chaotic_semantic_memory::singularity::Concept {
                id: format!("concept-{}", i),
                vector: HVec10240::random(),
                metadata: HashMap::new(),
                created_at: i as u64,
                modified_at: i as u64,
                expires_at: None,
                canonical_concept_ids: Vec::new(),
            };
            singularity.inject(concept).unwrap();
        }
        prop_assert!(singularity.len() <= max_concepts);
    }

    #[test]
    fn spectral_radius_valid_accepted(radius in 0.9f32..=1.1f32) {
        let mut reservoir = Reservoir::new_seeded(8, 64, 42).unwrap();
        let result = reservoir.set_spectral_radius(radius);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn spectral_radius_invalid_rejected(
        radius in prop_oneof![0.0f32..0.89f32, 1.11f32..2.0f32]
    ) {
        let mut reservoir = Reservoir::new_seeded(8, 64, 42).unwrap();
        let result = reservoir.set_spectral_radius(radius);
        prop_assert!(result.is_err());
    }

    #[test]
    fn reservoir_step_input_accepted(size in 1usize..100) {
        let mut reservoir = Reservoir::new_seeded(size, 64, 42).unwrap();
        let input = vec![0.5f32; size];
        let result = reservoir.step(&input);
        prop_assert!(result.is_ok());
    }
}

#[test]
fn concept_id_empty_rejected_via_api_regular() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let framework = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        framework.inject_concept("", HVec10240::random()).await
    });
    assert!(result.is_err());
}

#[test]
fn reservoir_input_size_zero_rejected() {
    let result = Reservoir::new_seeded(0, 64, 42);
    assert!(result.is_err());
}

#[test]
fn reservoir_size_zero_rejected() {
    let result = Reservoir::new_seeded(8, 0, 42);
    assert!(result.is_err());
}
