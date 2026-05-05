//! Property-based tests for hypervector and singularity operations.
//!
//! Float comparisons allowed: proptest assertions for exact association strengths.

#![allow(clippy::float_cmp)]

use std::collections::HashMap;

use chaotic_semantic_memory::hyperdim::HVec10240;
use chaotic_semantic_memory::singularity::{Concept, Singularity, SingularityConfig};
use proptest::prelude::*;

fn hvec_from_bytes(bytes: &[u8]) -> HVec10240 {
    HVec10240::from_bytes(bytes).expect("strategy always yields 1280-byte vectors")
}

proptest! {
    #[test]
    fn hypervector_roundtrip_from_and_to_bytes(bytes in proptest::collection::vec(any::<u8>(), 1280)) {
        let vector = hvec_from_bytes(&bytes);
        let recovered = HVec10240::from_bytes(&vector.to_bytes()).unwrap();
        prop_assert_eq!(recovered, vector);
    }

    #[test]
    fn cosine_similarity_stays_within_bounds(
        a_bytes in proptest::collection::vec(any::<u8>(), 1280),
        b_bytes in proptest::collection::vec(any::<u8>(), 1280),
    ) {
        let a = hvec_from_bytes(&a_bytes);
        let b = hvec_from_bytes(&b_bytes);

        let similarity = a.cosine_similarity(&b);
        prop_assert!(similarity >= -1.0);
        prop_assert!(similarity <= 1.0);
    }

    #[test]
    fn bundling_is_order_invariant_for_three_vectors(
        a_bytes in proptest::collection::vec(any::<u8>(), 1280),
        b_bytes in proptest::collection::vec(any::<u8>(), 1280),
        c_bytes in proptest::collection::vec(any::<u8>(), 1280),
    ) {
        let a = hvec_from_bytes(&a_bytes);
        let b = hvec_from_bytes(&b_bytes);
        let c = hvec_from_bytes(&c_bytes);

        let abc = HVec10240::bundle(&[a, b, c]).unwrap();
        let bca = HVec10240::bundle(&[b, c, a]).unwrap();
        prop_assert_eq!(abc, bca);
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

        singularity.inject(concept_a).unwrap();
        singularity.inject(concept_b).unwrap();
        singularity.associate("a", "b", strength).unwrap();

        let links = singularity.get_associations("a");
        prop_assert_eq!(links.len(), 1);
        prop_assert_eq!(links[0].0.as_str(), "b");
        prop_assert_eq!(links[0].1, strength);
    }
}
