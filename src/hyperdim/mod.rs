//! Hyperdimensional computing primitives
//!
//! Provides the [`Hypervector`] trait and implementations:
//! - [`HVec10240`]: 10240-bit f32-mapped hypervector (default)
//! - [`BHVec10240`]: 10240-bit binary-packed hypervector (opt-in)

pub mod batch;
pub mod binary;
pub mod hvec;
pub mod serde;
pub mod simd;

pub use batch::batch_cosine_similarity;
pub use binary::BHVec10240;
pub use hvec::HVec10240;

use crate::error::Result;
use std::fmt::Debug;

/// Trait for hyperdimensional vectors.
pub trait Hypervector:
    Sized
    + Clone
    + Copy
    + Debug
    + Send
    + Sync
    + PartialEq
    + ::serde::Serialize
    + for<'de> ::serde::Deserialize<'de>
{
    /// Dimension of the hypervector.
    const DIMENSION: usize;

    /// Create a zero hypervector.
    fn zero() -> Self;

    /// Create a random hypervector.
    fn random() -> Self;

    /// XOR binding of two hypervectors.
    fn bind(&self, other: &Self) -> Self;

    /// Bundle multiple hypervectors.
    fn bundle(vectors: &[Self]) -> Result<Self>;

    /// Cyclic permutation.
    fn permute(&self, shift: usize) -> Self;

    /// Cosine similarity (1.0 = identical, 0.0 = orthogonal, -1.0 = opposite).
    fn cosine_similarity(&self, other: &Self) -> f32;

    /// Hamming distance (number of differing bits).
    fn hamming_distance(&self, other: &Self) -> u32;

    /// Convert to bytes.
    fn to_bytes(&self) -> Vec<u8>;

    /// Create from bytes.
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}
