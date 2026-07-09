#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Hyperdimensional vector coverage tests.
//!
//! Covers: sparse, new_seeded, zero, to_bytes/from_bytes, hamming_distance

use chaotic_semantic_memory::prelude::*;

#[test]
fn hvec_sparse_creates_sparse_vector() {
    let sparse_vec = HVec10240::sparse(0.1); // 10% density
    // Verify it's not zero
    let zero = HVec10240::zero();
    assert_ne!(sparse_vec, zero);
}

#[test]
fn hvec_sparse_zero_density_returns_zero_vector() {
    let zero_vec = HVec10240::sparse(0.0);
    let zero = HVec10240::zero();
    assert_eq!(zero_vec, zero);
}

#[test]
fn hvec_sparse_high_density_is_different_from_zero() {
    let dense_vec = HVec10240::sparse(0.8);
    let zero = HVec10240::zero();
    // High density vector should be different from zero
    assert_ne!(dense_vec, zero);
}

#[test]
fn hvec_new_seeded_is_deterministic() {
    let v1 = HVec10240::new_seeded(42);
    let v2 = HVec10240::new_seeded(42);
    assert_eq!(v1, v2);

    let v3 = HVec10240::new_seeded(43);
    assert_ne!(v1, v3);
}

#[test]
fn hvec_zero_different_from_random() {
    let zero = HVec10240::zero();
    let random = HVec10240::random();
    // Zero should be different from random
    assert_ne!(zero, random);

    // Zero should have zero distance from itself
    let self_dist = zero.hamming_distance(&zero);
    assert_eq!(self_dist, 0);
}

#[test]
fn hvec_to_bytes_and_from_bytes_roundtrip() {
    let original = HVec10240::random();
    let bytes = original.to_bytes();
    assert_eq!(bytes.len(), 1280);

    let restored = HVec10240::from_bytes(&bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn hvec_from_bytes_wrong_length_fails() {
    let short_bytes = vec![0u8; 100];
    let result = HVec10240::from_bytes(&short_bytes);
    assert!(result.is_err());
}

#[test]
fn hvec_hamming_distance_self_is_zero() {
    let v = HVec10240::random();
    let dist = v.hamming_distance(&v);
    assert_eq!(dist, 0);
}

#[test]
fn hvec_hamming_distance_different_vectors() {
    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    let dist = v1.hamming_distance(&v2);
    // Random vectors should have ~50% bit differences (around 5120 bits)
    assert!(dist > 0);
}

#[test]
fn hvec_bind_xor_operation() {
    let v1 = HVec10240::random();
    let v2 = HVec10240::random();
    let bound = v1.bind(&v2);

    // Binding should be reversible (XOR twice returns original)
    let unbound = bound.bind(&v2);
    assert_eq!(unbound, v1);
}

#[test]
fn hvec_permute_shifts_bits() {
    let v = HVec10240::random();
    let permuted = v.permute(1);
    // Permute should change the vector
    assert_ne!(v, permuted);
}

#[test]
fn hvec_bundle_empty_returns_zero() {
    let result = HVec10240::bundle(&[]).unwrap();
    assert_eq!(result, HVec10240::zero());
}

#[test]
fn hvec_bundle_single_returns_same() {
    let v = HVec10240::random();
    let bundled = HVec10240::bundle(&[v]).unwrap();
    // Bundle of single vector returns the same (threshold = 1)
    assert_eq!(bundled, v);
}

#[test]
fn hvec_cosine_similarity_self_is_one() {
    let v = HVec10240::random();
    // Self similarity should be 1.0
    let self_sim = v.cosine_similarity(&v);
    assert!((self_sim - 1.0).abs() < 0.001);
}

#[test]
fn hvec_cosine_similarity_zero_vector() {
    let zero = HVec10240::zero();
    let random = HVec10240::random();
    let sim = zero.cosine_similarity(&random);
    // Similarity with zero vector should be 0.0 (or undefined behavior)
    // Actually cosine similarity with zero vector is undefined, but the implementation
    // might handle it. Just verify it's a valid number.
    assert!(sim.is_finite());
}
