use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::reservoir::Reservoir;
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};
use std::collections::HashMap;

#[tokio::test]
async fn empty_sequence_returns_zero_hypervector() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .with_reservoir_size(HVec10240::DIMENSION)
        .build()
        .await
        .unwrap();

    let output = framework.process_sequence(&[]).await.unwrap();
    assert_eq!(output, HVec10240::zero());
}

#[test]
fn zero_length_inputs_are_rejected() {
    let mut reservoir = Reservoir::new_seeded(8, 64, 7).unwrap();
    assert!(reservoir.step(&[]).is_err());
    assert!(Reservoir::new_seeded(0, 64, 7).is_err());
}

#[test]
fn concept_and_association_limits_enforced() {
    let mut singularity = Singularity::with_config(SingularityConfig {
        max_concepts: Some(2),
        max_associations_per_concept: Some(1),
        concept_cache_size: 64,
        max_cached_top_k: 100,
    });

    let mk = |id: &str, created_at: u64| Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at,
        modified_at: created_at,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    singularity.inject(mk("a", 1)).unwrap();
    singularity.inject(mk("b", 2)).unwrap();
    singularity.inject(mk("c", 3)).unwrap();

    assert_eq!(singularity.len(), 2);
    assert!(singularity.get("a").is_none());
    assert!(singularity.get("b").is_some());
    assert!(singularity.get("c").is_some());

    singularity.associate("b", "c", 0.9).unwrap();
    singularity.associate("b", "b", 0.1).unwrap();
    let links = singularity.get_associations("b");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].0, "c");
}

#[test]
fn singularity_association_strength_is_validated() {
    let mut singularity = Singularity::new();
    let mk = |id: &str| Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    singularity.inject(mk("a")).unwrap();
    singularity.inject(mk("b")).unwrap();

    assert!(singularity.associate("a", "b", f32::NAN).is_err());
    assert!(singularity.associate("a", "b", f32::INFINITY).is_err());
    assert!(singularity.associate("a", "b", -0.1).is_err());
    assert!(singularity.associate("a", "b", 1.1).is_err());
}

#[test]
fn singularity_association_updates_instead_of_duplicating() {
    let mut singularity = Singularity::new();
    let mk = |id: &str| Concept {
        id: id.to_string(),
        vector: HVec10240::random(),
        metadata: HashMap::new(),
        created_at: 1,
        modified_at: 1,
        expires_at: None,
        canonical_concept_ids: Vec::new(),
    };

    singularity.inject(mk("a")).unwrap();
    singularity.inject(mk("b")).unwrap();

    singularity.associate("a", "b", 0.2).unwrap();
    singularity.associate("a", "b", 0.9).unwrap();

    let links = singularity.get_associations("a");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].0, "b");
    assert_eq!(links[0].1, 0.9);
}

#[test]
fn spectral_radius_inclusive_bounds() {
    let mut reservoir = Reservoir::new_seeded(8, 64, 11).unwrap();
    assert!(reservoir.set_spectral_radius(0.9).is_ok());
    assert!(reservoir.set_spectral_radius(1.1).is_ok());
    assert!(reservoir.set_spectral_radius(0.8999).is_err());
    assert!(reservoir.set_spectral_radius(1.1001).is_err());
}

#[test]
fn reservoir_size_boundaries() {
    let small = Reservoir::new_seeded(1, 1, 3).unwrap();
    assert_eq!(small.size(), 1);
    assert!(small.to_hypervector().is_err());

    let mut exact = Reservoir::new_seeded(1, HVec10240::DIMENSION, 3).unwrap();
    exact.step(&[0.25]).unwrap();
    assert!(exact.to_hypervector().is_ok());
}
