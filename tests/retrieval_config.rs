#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Retrieval config validation tests for coverage gap in singularity_retrieval.rs.
//!
//! Covers: RetrievalConfig::validate, set_retrieval_config, FilterStrategy branches
//!
//! Float comparisons allowed: test assertions for exact default values.

#![allow(clippy::float_cmp)]

use chaotic_semantic_memory::HVec10240;
use chaotic_semantic_memory::prelude::*;
use chaotic_semantic_memory::singularity::{Singularity, SingularityConfig};

const NS: &str = "_default";

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
        bm25_absence_short_circuit: false,
        early_exit_hdc: 0.85,
        bridge_expand_cap: 32,
    };
    config.validate().unwrap();
}

#[test]
fn retrieval_config_reject_zero_max_candidates() {
    let config = RetrievalConfig {
        max_candidates: 0,
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn retrieval_config_reject_invalid_early_exit_hdc() {
    let config_negative = RetrievalConfig {
        early_exit_hdc: -0.1,
        ..Default::default()
    };
    assert!(config_negative.validate().is_err());

    let config_too_high = RetrievalConfig {
        early_exit_hdc: 1.1,
        ..Default::default()
    };
    assert!(config_too_high.validate().is_err());

    let config_nan = RetrievalConfig {
        early_exit_hdc: f32::NAN,
        ..Default::default()
    };
    assert!(config_nan.validate().is_err());
}

#[test]
fn retrieval_config_for_token_count_short_query() {
    let config = RetrievalConfig::for_token_count(1);
    assert_eq!(config.max_candidates, 64);
    assert_eq!(config.graph_depth, 0);
    assert!(!config.enable_graph_candidates);

    let config_zero = RetrievalConfig::for_token_count(0);
    assert_eq!(config_zero.max_candidates, 64);
    assert_eq!(config_zero.graph_depth, 0);
    assert!(!config_zero.enable_graph_candidates);
}

#[test]
fn retrieval_config_for_token_count_medium_query() {
    let config = RetrievalConfig::for_token_count(3);
    assert_eq!(config.max_candidates, 128);
    assert_eq!(config.graph_depth, 1);
    assert_eq!(config.graph_fanout, 4);

    let config_four = RetrievalConfig::for_token_count(4);
    assert_eq!(config_four.max_candidates, 128);
    assert_eq!(config_four.graph_depth, 1);
    assert_eq!(config_four.graph_fanout, 4);
}

#[test]
fn retrieval_config_for_token_count_long_query_matches_default() {
    let config = RetrievalConfig::for_token_count(8);
    let default_config = RetrievalConfig::default();

    assert_eq!(config.max_candidates, default_config.max_candidates);
    assert_eq!(config.graph_depth, default_config.graph_depth);
    assert_eq!(config.graph_fanout, default_config.graph_fanout);
    assert_eq!(config.enable_graph_candidates, default_config.enable_graph_candidates);
    assert_eq!(config.bm25_absence_short_circuit, default_config.bm25_absence_short_circuit);
    assert_eq!(config.early_exit_hdc, default_config.early_exit_hdc);
    assert_eq!(config.bridge_expand_cap, default_config.bridge_expand_cap);
}

#[test]
fn singularity_set_retrieval_config_accepts_valid() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
    let config = RetrievalConfig {
        max_candidates: 2000,
        ..Default::default()
    };
    sing.set_retrieval_config(config).unwrap();
}

#[test]
fn singularity_set_retrieval_config_rejects_invalid_bucket_width() {
    let mut sing = Singularity::<HVec10240>::new(SingularityConfig::default());
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
