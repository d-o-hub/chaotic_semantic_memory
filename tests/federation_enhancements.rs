#[cfg(test)]
mod federation_tests {
    use chaotic_semantic_memory::prelude::*;

    #[test]
    fn test_hvec_hash_available() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let v1 = HVec10240::random();
        let v2 = HVec10240::random();
        set.insert(v1);
        set.insert(v2);
        assert!(set.contains(&v1));
        assert!(set.contains(&v2));
    }

    #[test]
    fn test_wire_versions() {
        assert_eq!(HVec10240::WIRE_VERSION, 1);
        assert_eq!(BundleAccumulator::WIRE_VERSION, 1);
        assert_eq!(ConceptGraph::WIRE_VERSION, 1);
    }

    #[test]
    #[cfg(feature = "signing")]
    fn test_hvec_canonical_bytes() {
        let v = HVec10240::random();
        let canon = v.canonical_bytes();
        assert_eq!(canon.len(), 4 + 1280);
        assert_eq!(&canon[0..4], &HVec10240::WIRE_VERSION.to_le_bytes());
        assert_eq!(&canon[4..], &v.to_bytes()[..]);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_bundle_accumulator_serde_roundtrip() {
        let mut acc = BundleAccumulator::new();
        let v1 = HVec10240::random();
        acc.add(&v1);

        let json = serde_json::to_string(&acc).unwrap();
        let decoded: BundleAccumulator = serde_json::from_str(&json).unwrap();

        assert_eq!(acc.len(), decoded.len());
        assert_eq!(acc.finalize(), decoded.finalize());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_concept_graph_serde_roundtrip() {
        let mut graph = ConceptGraph::new();
        graph.add_concept(
            chaotic_semantic_memory::semantic_bridge::CanonicalConcept::new("c1").with_label("l1"),
        );

        let json = serde_json::to_string(&graph).unwrap();
        let decoded: ConceptGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(graph.concept_count(), decoded.concept_count());
        assert!(decoded.get_concept("c1").is_some());
    }
}
