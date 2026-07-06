use super::RetrievalConfig;
use crate::singularity::{Singularity, SingularityConfig};
use csm_core::hyperdim::HVec10240;

#[test]
fn singularity_last_stats_v2() {
    let s = Singularity::<HVec10240>::new(SingularityConfig::default());
    assert_eq!(s.last_retrieval_stats("_default").candidate_count, 0);
}

#[test]
fn singularity_get_config_v2() {
    let s = Singularity::<HVec10240>::new(SingularityConfig::default());
    assert_eq!(s.retrieval_config().max_candidates, 1000);
}

#[test]
fn test_generate_graph_candidates_logic() {
    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());
    let ns = "test";

    // Inject 5 concepts: seed -> {c1, c2, c3, c4}
    let seed = crate::singularity::ConceptBuilder::new("seed")
        .build()
        .unwrap();
    let c1 = crate::singularity::ConceptBuilder::new("c1")
        .build()
        .unwrap();
    let c2 = crate::singularity::ConceptBuilder::new("c2")
        .build()
        .unwrap();
    let c3 = crate::singularity::ConceptBuilder::new("c3")
        .build()
        .unwrap();
    let c4 = crate::singularity::ConceptBuilder::new("c4")
        .build()
        .unwrap();

    s.inject(ns, seed).unwrap();
    s.inject(ns, c1).unwrap();
    s.inject(ns, c2).unwrap();
    s.inject(ns, c3).unwrap();
    s.inject(ns, c4).unwrap();

    // Associations with different strengths
    s.associate(ns, "seed", "c1", 0.1).unwrap();
    s.associate(ns, "seed", "c2", 0.9).unwrap();
    s.associate(ns, "seed", "c3", 0.5).unwrap();
    s.associate(ns, "seed", "c4", 0.7).unwrap();

    // Set fanout to 2. Should pick c2 (0.9) and c4 (0.7)
    let mut config = RetrievalConfig::default();
    config.graph_fanout = 2;
    config.graph_depth = 1;
    s.set_retrieval_config(config).unwrap();

    // We need to make sure seed is the closest.
    // For simplicity in test, let's just use seed's vector as query
    let seed_vec = s.get(ns, "seed").unwrap().vector;

    let candidates = s.generate_graph_candidates(ns, &seed_vec);

    // Candidates should contain seed, c2, and c4.
    let names: std::collections::HashSet<_> = candidates
        .into_iter()
        .map(|idx| s.get_namespace(ns).unwrap().concept_indices[idx].clone())
        .collect();

    assert!(names.contains("seed"));
    assert!(names.contains("c2"));
    assert!(names.contains("c4"));
    assert!(!names.contains("c1"));
    assert!(!names.contains("c3"));
    assert_eq!(names.len(), 3);
}
