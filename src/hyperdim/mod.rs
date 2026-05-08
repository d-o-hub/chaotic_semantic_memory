pub mod batch;
pub mod hvec;
pub mod simd;

pub use batch::batch_cosine_similarity;
pub use hvec::HVec10240;

use crate::error::Result;
use std::fmt::Debug;

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
}
#[cfg(feature = "hv-binary")]
pub mod binary;
#[cfg(feature = "hv-binary")]
pub use binary::BHVec10240;
