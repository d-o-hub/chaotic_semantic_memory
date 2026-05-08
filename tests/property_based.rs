//! Property-based tests for hypervector and singularity operations.
//!
//! Float comparisons allowed: proptest assertions for exact association strengths.

#![allow(clippy::float_cmp)]

use std::collections::HashMap;

use chaotic_semantic_memory::hyperdim::{HVec10240, Hypervector};
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};
use proptest::prelude::*;

const NS: &str = "_default";

/// Generate 10240 f32 values in [-1.0, 1.0) to avoid NaN and maintain valid HVec semantics.
fn valid_f32s() -> impl Strategy<Value = Vec<f32>> {
    proptest::collection::vec(-1.0f32..1.0f32, 10240)
}

fn hvec_from_f32s(data: &[f32; 10240]) -> HVec10240 {
    HVec10240::from_f32_array(data)
}

proptest! {
    #[test]
    fn hypervector_roundtrip_from_and_to_bytes(data in valid_f32s()) {
        let data: [f32; 10240] = data.try_into().unwrap();
        let vector = hvec_from_f32s(&data);
        let recovered = HVec10240::from_bytes(&vector.to_bytes()).unwrap();
        prop_assert_eq!(recovered, vector);
    }

    #[test]
    fn cosine_similarity_stays_within_bounds(
        a_data in valid_f32s(),
        b_data in valid_f32s(),
    ) {
        let a_data: [f32; 10240] = a_data.try_into().unwrap();
        let b_data: [f32; 10240] = b_data.try_into().unwrap();
        let a = hvec_from_f32s(&a_data);
        let b = hvec_from_f32s(&b_data);

        let similarity = a.cosine_similarity(&b);
        prop_assert!(similarity >= -1.0);
        prop_assert!(similarity <= 1.0);
    }

    #[test]
    fn bundling_is_order_invariant_for_three_vectors(
        a_data in valid_f32s(),
        b_data in valid_f32s(),
        c_data in valid_f32s(),
    ) {
        let a_data: [f32; 10240] = a_data.try_into().unwrap();
        let b_data: [f32; 10240] = b_data.try_into().unwrap();
        let c_data: [f32; 10240] = c_data.try_into().unwrap();
        let a = hvec_from_f32s(&a_data);
        let b = hvec_from_f32s(&b_data);
        let c = hvec_from_f32s(&c_data);

        // f32 addition is commutative but not associative, so three-element
        // bundles may differ by ~2e-7 due to summation order. Use cosine
        // similarity to verify near-equality instead of exact comparison.
        let abc = HVec10240::bundle(&[a, b, c]).unwrap();
        let bca = HVec10240::bundle(&[b, c, a]).unwrap();
        let cos = abc.cosine_similarity(&bca);
        prop_assert!(
            cos > 0.9999,
            "reordered bundles should be nearly identical, got cos={}",
            cos
        );
    }

    #[test]
    fn associate_creates_queryable_link(strength in 0.0f32..=1.0f32) {
        let mut singularity = Singularity::new(SingularityConfig::default());
        let concept_a = Concept {
            id: "a".to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };
        let concept_b = Concept {
            id: "b".to_string(),
            vector: HVec10240::random(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: Vec::new(),
        };

        singularity.inject(NS, concept_a).unwrap();
        singularity.inject(NS, concept_b).unwrap();
        singularity.associate(NS, "a", "b", strength).unwrap();

        let links = singularity.get_associations(NS, "a");
        prop_assert_eq!(links.len(), 1);
        prop_assert_eq!(links[0].0.as_str(), "b");
        prop_assert_eq!(links[0].1, strength);
    }
}
