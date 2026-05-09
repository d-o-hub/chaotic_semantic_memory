pub mod batch;
pub mod hvec;
pub mod simd;

pub use batch::batch_cosine_similarity;
pub use hvec::HVec10240;

use crate::error::Result;
use std::fmt::Debug;
use serde::{Serialize, Deserialize};

pub trait Hypervector:
    Sized
    + Clone
    + Copy
    + Debug
    + Send
    + Sync
    + PartialEq
    + Serialize
    + for<'de> Deserialize<'de>
    + 'static
{
    const DIMENSION: usize;
    fn format_name() -> &'static str;
    fn zero() -> Self;
    fn random() -> Self;
    fn bind(&self, other: &Self) -> Self;
    fn bundle(vectors: &[Self]) -> Result<Self>;
    fn permute(&self, shift: usize) -> Self;
    fn cosine_similarity(&self, other: &Self) -> f32;
    fn hamming_distance(&self, other: &Self) -> u32;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self>;

    /// Convert from the standard f32 hypervector representation.
    fn from_hvec(v: &HVec10240) -> Self;

    /// Convert to the standard f32 hypervector representation.
    fn to_hvec(&self) -> HVec10240;

    /// Get bit at position (used by LSH).
    fn get_bit(&self, pos: usize) -> bool;
}

#[cfg(feature = "hv-binary")]
pub mod binary;
#[cfg(feature = "hv-binary")]
pub use binary::BHVec10240;
