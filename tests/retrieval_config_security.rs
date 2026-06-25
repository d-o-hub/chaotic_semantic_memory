use chaotic_semantic_memory::RetrievalConfig;
use chaotic_semantic_memory::singularity::{Singularity, SingularityConfig};
use csm_core::hyperdim::HVec10240;

#[test]
fn test_retrieval_config_validation_security_limits() {
    // 1. Test max_candidates limit
    let config = RetrievalConfig {
        max_candidates: 100_001,
        ..RetrievalConfig::default()
    };
    assert!(config.validate().is_err());

    // 2. Test graph_depth limit
    let config = RetrievalConfig {
        graph_depth: 33,
        ..RetrievalConfig::default()
    };
    assert!(config.validate().is_err());

    // 3. Test graph_fanout limit
    let config = RetrievalConfig {
        graph_fanout: 10_001,
        ..RetrievalConfig::default()
    };
    assert!(config.validate().is_err());

    // 4. Test bucket_probe_width limit (existing)
    let config = RetrievalConfig {
        bucket_probe_width: 17,
        ..RetrievalConfig::default()
    };
    assert!(config.validate().is_err());

    // 5. Test valid config
    let config = RetrievalConfig {
        max_candidates: 100_000,
        graph_depth: 32,
        graph_fanout: 10_000,
        bucket_probe_width: 16,
        ..RetrievalConfig::default()
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_singularity_set_retrieval_config_enforces_validation() {
    let mut s = Singularity::<HVec10240>::new(SingularityConfig::default());

    let invalid_config = RetrievalConfig {
        max_candidates: 200_000,
        ..RetrievalConfig::default()
    };

    let result = s.set_retrieval_config(invalid_config);
    assert!(result.is_err());
}
