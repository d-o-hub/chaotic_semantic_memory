//! Hyperdimensional computing operations and trait abstractions.
//!
//! Contains the `Hypervector` trait definition, its implementation for `HVec10240`,
//! and scalar helper functions used by the bundle algorithm.

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

use crate::error::Result;
use crate::hyperdim::HVec10240;

/// Common interface for hypervectors
pub trait Hypervector:
    Debug + Clone + Copy + PartialEq + Eq + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>
{
    const DIMENSION: usize;
    const FORMAT_NAME: &'static str;
    fn zero() -> Self;
    fn random() -> Self;
    fn new_seeded(seed: u64) -> Self;
    fn bundle(vectors: &[&Self]) -> Result<Self>;
    fn bind(&self, other: &Self) -> Self;
    fn cosine_similarity(&self, other: &Self) -> f32;
    fn hamming_distance(&self, other: &Self) -> u32;
    fn permute(&self, shift: usize) -> Self;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

impl Hypervector for HVec10240 {
    const DIMENSION: usize = 10240;
    const FORMAT_NAME: &'static str = "f32";

    fn zero() -> Self {
        Self::zero()
    }

    fn random() -> Self {
        Self::random()
    }

    fn new_seeded(seed: u64) -> Self {
        Self::new_seeded(seed)
    }

    fn bundle(vectors: &[&Self]) -> Result<Self> {
        // HVec10240::bundle expects &[Self], but trait gives &[&Self]
        let owned_vecs: Vec<Self> = vectors.iter().map(|&v| *v).collect();
        Self::bundle(&owned_vecs)
    }

    fn bind(&self, other: &Self) -> Self {
        self.bind(other)
    }

    fn cosine_similarity(&self, other: &Self) -> f32 {
        self.cosine_similarity(other)
    }

    fn hamming_distance(&self, other: &Self) -> u32 {
        self.hamming_distance(other)
    }

    fn permute(&self, shift: usize) -> Self {
        self.permute(shift)
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::from_bytes(bytes)
    }
}

/// Scalar bit-sliced addition for a single word.
///
/// Centralized helper for sequential and parallel fallback paths.
#[allow(dead_code)]
pub fn bundle_word_scalar(
    vectors: &[HVec10240],
    word_idx: usize,
    threshold: usize,
    num_planes: usize,
) -> u128 {
    let mut planes = [0u128; 64];
    for v in vectors {
        let mut carry = v.data[word_idx];
        for plane in planes.iter_mut().take(num_planes) {
            let next_carry = *plane & carry;
            *plane ^= carry;
            carry = next_carry;
            if carry == 0 {
                break;
            }
        }
    }
    let (mut current_eq, mut current_gt) = (!0u128, 0u128);
    for p in (0..num_planes).rev() {
        if ((threshold >> p) & 1) == 1 {
            current_eq &= planes[p];
        } else {
            current_gt |= current_eq & planes[p];
            current_eq &= !planes[p];
        }
    }
    current_gt | current_eq
}
