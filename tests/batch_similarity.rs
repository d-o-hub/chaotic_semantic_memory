//! Integration tests for batch_cosine_similarity function
//!
//! Tests verify that the optimized batched similarity computation produces
//! identical results to individual cosine_similarity calls.

use chaotic_semantic_memory::batch_cosine_similarity;
use chaotic_semantic_memory::prelude::*;

/// Tolerance for floating point comparison between batch and individual results.
const SIMILARITY_EPSILON: f32 = 1e-6;

#[test]
fn test_batch_similarity_matches_individual() {
    let query = HVec10240::random();
    let candidates: Vec<_> = (0..1000).map(|_| HVec10240::random()).collect();

    let batch_results = batch_cosine_similarity(&query, &candidates);
    let individual_results: Vec<f32> = candidates
        .iter()
        .map(|c| query.cosine_similarity(c))
        .collect();

    assert_eq!(
        batch_results.len(),
        individual_results.len(),
        "Batch result count should match candidate count"
    );
    for (i, (batch, individual)) in batch_results
        .iter()
        .zip(individual_results.iter())
        .enumerate()
    {
        assert!(
            (batch - individual).abs() < SIMILARITY_EPSILON,
            "Batch result {} doesn't match individual {} at index {} (diff: {})",
            batch,
            individual,
            i,
            (batch - individual).abs()
        );
    }
}

#[test]
fn test_batch_similarity_empty() {
    let query = HVec10240::random();
    let candidates: Vec<HVec10240> = vec![];

    let results = batch_cosine_similarity(&query, &candidates);

    assert!(
        results.is_empty(),
        "Empty candidates should return empty results"
    );
}

#[test]
fn test_batch_similarity_single() {
    let query = HVec10240::random();
    let candidate = HVec10240::random();

    let batch_results = batch_cosine_similarity(&query, &[candidate]);
    let individual_result = query.cosine_similarity(&candidate);

    assert_eq!(
        batch_results.len(),
        1,
        "Single candidate should return single result"
    );
    assert!(
        (batch_results[0] - individual_result).abs() < SIMILARITY_EPSILON,
        "Batch result {} should match individual result {}",
        batch_results[0],
        individual_result
    );
}

#[test]
fn test_batch_similarity_odd_count() {
    let query = HVec10240::random();
    // Odd number of candidates (not divisible by chunk size 64)
    let candidates: Vec<_> = (0..127).map(|_| HVec10240::random()).collect();

    let batch_results = batch_cosine_similarity(&query, &candidates);
    let individual_results: Vec<f32> = candidates
        .iter()
        .map(|c| query.cosine_similarity(c))
        .collect();

    assert_eq!(
        batch_results.len(),
        candidates.len(),
        "Odd count batch results length should match candidate count"
    );
    for (i, (batch, individual)) in batch_results
        .iter()
        .zip(individual_results.iter())
        .enumerate()
    {
        assert!(
            (batch - individual).abs() < SIMILARITY_EPSILON,
            "Batch result {} doesn't match individual {} at index {} (odd count)",
            batch,
            individual,
            i
        );
    }
}
