#[cfg(test)]
mod tests_v2 {
    use super::*;
    use crate::singularity::{ConceptBuilder, Singularity, SingularityConfig};
    use csm_core_lib::hyperdim::HVec10240;

    #[test]
    fn singularity_last_stats_v2() {
        let s = Singularity::<HVec10240>::new(SingularityConfig::default());
        assert_eq!(s.last_retrieval_stats("_default").candidate_count, 0);
    }

    #[test]
    fn singularity_get_config_v2() {
        let s = Singularity::<HVec10240>::new(SingularityConfig::default());
        assert_eq!(s.retrieval_config().max_candidates, 256);
    }

    #[test]
    fn test_generate_graph_candidates_logic() {
        let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
        let mut config = RetrievalConfig::default();
        config.enable_graph_candidates = true;
        config.graph_depth = 1;
        config.graph_fanout = 2;
        s.set_retrieval_config(config).unwrap();

        let v1 = HVec10240::random();
        let v2 = HVec10240::random();
        let v3 = HVec10240::random();
        let v4 = HVec10240::random();

        s.inject(
            "_default",
            ConceptBuilder::new("c1")
                .with_vector(v1.clone())
                .build()
                .unwrap(),
        )
        .unwrap();
        s.inject(
            "_default",
            ConceptBuilder::new("c2").with_vector(v2).build().unwrap(),
        )
        .unwrap();
        s.inject(
            "_default",
            ConceptBuilder::new("c3").with_vector(v3).build().unwrap(),
        )
        .unwrap();
        s.inject(
            "_default",
            ConceptBuilder::new("c4").with_vector(v4).build().unwrap(),
        )
        .unwrap();

        // c1 -> c2 (0.9), c1 -> c3 (0.8), c1 -> c4 (0.1)
        s.associate("_default", "c1", "c2", 0.9).unwrap();
        s.associate("_default", "c1", "c3", 0.8).unwrap();
        s.associate("_default", "c1", "c4", 0.1).unwrap();

        let candidates = s.generate_graph_candidates("_default", &v1);
        // c1 is seed, c2 and c3 are top 2 neighbors. c4 is excluded by fanout=2.
        assert_eq!(candidates.len(), 3);

        let ns_state = s.get_namespace("_default").unwrap();
        let ids: std::collections::HashSet<_> = candidates
            .iter()
            .map(|&idx| ns_state.concept_indices[idx].as_str())
            .collect();
        assert!(ids.contains("c1"));
        assert!(ids.contains("c2"));
        assert!(ids.contains("c3"));
        assert!(!ids.contains("c4"));
    }
}
