//! Retrieval config validation tests for coverage gap in singularity_retrieval.rs.
//!
//! Covers: RetrievalConfig::validate, set_retrieval_config, FilterStrategy branches

use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::Singularity;

#[test]
fn retrieval_config_default_is_valid() {
    let config = RetrievalConfig::default();
    config.validate().unwrap();
}

#[test]
fn retrieval_config_bucket_probe_width_exceeds_limit_invalid() {
    let config = RetrievalConfig {
        bucket_probe_width: 100, // Exceeds MAX_BUCKET_PROBE_WIDTH (likely 10)
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn retrieval_config_valid_custom_values() {
    let config = RetrievalConfig {
        max_candidates: 500,
        candidate_ratio_fallback: 0.7,
        graph_depth: 3,
        graph_fanout: 20,
        bucket_probe_width: 4,
        enable_graph_candidates: true,
        enable_bucket_candidates: true,
    };
    config.validate().unwrap();
}

#[test]
fn singularity_set_retrieval_config_accepts_valid() {
    let mut sing = Singularity::new();
    let config = RetrievalConfig {
        max_candidates: 2000,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();
}

#[test]
fn singularity_set_retrieval_config_rejects_invalid_bucket_width() {
    let mut sing = Singularity::new();
    let config = RetrievalConfig {
        bucket_probe_width: 1000, // Invalid
        ..Default::default()
    };
    assert!(sing.set_retrieval_config(config).is_err());
}

#[test]
fn retrieval_stats_snapshot_default() {
    let stats = RetrievalStats::default();
    assert_eq!(stats.candidate_count, 0);
    assert_eq!(stats.scored_count, 0);
    assert!(!stats.fell_back_to_exact_scan);
    assert_eq!(stats.selectivity_ratio, 0.0);
    assert!(stats.filter_strategy.is_none());
}

#[test]
fn candidate_source_variants() {
    // Exercise CandidateSource enum for coverage
    assert_eq!(CandidateSource::Metadata, CandidateSource::Metadata);
    assert_ne!(CandidateSource::Metadata, CandidateSource::Graph);
    assert_ne!(CandidateSource::Bucket, CandidateSource::ExactFallback);
}

#[test]
fn filter_strategy_variants() {
    // Exercise FilterStrategy enum for coverage
    assert_eq!(FilterStrategy::Pre, FilterStrategy::Pre);
    assert_ne!(FilterStrategy::Pre, FilterStrategy::BucketPost);
    assert_ne!(FilterStrategy::BucketPost, FilterStrategy::ScanPost);
}
