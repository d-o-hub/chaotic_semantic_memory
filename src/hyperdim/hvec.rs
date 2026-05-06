//! Hyperdimensional computing primitives
//!
//! Implements 10240-dimensional float hypervectors.
//! For 32x compressed binary hypervectors, see [`BHVec10240`](super::binary::BHVec10240).

// Casts are intentional for HDC dimension math
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use rand::RngExt;

use crate::error::Result;
use super::Hypervector;

/// 10240-dimensional float hypervector (40 KB)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HVec10240 {
    pub(crate) data: [f32; 10240],
}

// Manual Eq because f32 doesn't implement Eq, but for our purposes
// exact match of all lanes is enough for equality check in tests.
impl Eq for HVec10240 {}

impl HVec10240 {
    pub const DIMENSION: usize = 10240;

    /// Create a new hypervector with all zeros
    pub const fn zero_const() -> Self {
        Self { data: [0.0f32; 10240] }
    }

    /// Create a deterministic random hypervector from a seed.
    pub fn new_seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut data = [0.0f32; 10240];
        for val in &mut data {
            // Generate random values in range [-1, 1] for bipolar-like float HDC
            *val = rng.random_range(-1.0..1.0);
        }
        Self { data }
    }

    /// Create a random sparse hypervector with given density
    pub fn sparse(density: f32) -> Self {
        let mut rng = rand::rng();
        let mut data = [0.0f32; 10240];
        let bits_to_set = (Self::DIMENSION as f32 * density) as usize;

        for _ in 0..bits_to_set {
            let pos = rng.random_range(0..Self::DIMENSION);
            data[pos] = 1.0;
        }

        Self { data }
    }

    /// Create a zero hypervector.
    pub fn zero() -> Self {
        <Self as Hypervector>::zero()
    }

    /// Create a random hypervector.
    pub fn random() -> Self {
        <Self as Hypervector>::random()
    }

    /// XOR binding (binary) or element-wise multiplication (float) of two hypervectors.
    pub fn bind(&self, other: &Self) -> Self {
        <Self as Hypervector>::bind(self, other)
    }

    /// Bundle multiple hypervectors.
    pub fn bundle(vectors: &[Self]) -> Result<Self> {
        <Self as Hypervector>::bundle(vectors)
    }

    /// Cyclic permutation.
    pub fn permute(&self, shift: usize) -> Self {
        <Self as Hypervector>::permute(self, shift)
    }

    /// Cosine similarity.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        <Self as Hypervector>::cosine_similarity(self, other)
    }

    /// Hamming distance.
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        <Self as Hypervector>::hamming_distance(self, other)
    }

    /// Convert to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        <Self as Hypervector>::to_bytes(self)
    }

    /// Create from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        <Self as Hypervector>::from_bytes(bytes)
    }

    /// Set a specific dimension to 1.0.
    pub fn set_bit(&mut self, pos: usize) {
        if pos < 10240 {
            self.data[pos] = 1.0;
        }
    }

    /// Flip a specific dimension (1.0 -> -1.0 or vice versa).
    pub fn flip_bit(&mut self, pos: usize) {
        if pos < 10240 {
            self.data[pos] = -self.data[pos];
        }
    }
}

impl Hypervector for HVec10240 {
    const DIMENSION: usize = 10240;

    fn format_name() -> &'static str {
        "f32"
    }

    fn zero() -> Self {
        Self::zero_const()
    }

    fn random() -> Self {
        let mut rng = rand::rng();
        let mut data = [0.0f32; 10240];
        for val in &mut data {
            *val = rng.random_range(-1.0..1.0);
        }
        Self { data }
    }

    /// Binding for float vectors is element-wise multiplication.
    fn bind(&self, other: &Self) -> Self {
        let mut result = [0.0f32; 10240];
        for i in 0..10240 {
            result[i] = self.data[i] * other.data[i];
        }
        Self { data: result }
    }

    /// Bundling for float vectors is element-wise addition (superposition).
    fn bundle(vectors: &[Self]) -> Result<Self> {
        if vectors.is_empty() {
            return Ok(Self::zero());
        }
        let mut result = Box::new([0.0f32; 10240]);
        for v in vectors {
            for i in 0..10240 {
                result[i] += v.data[i];
            }
        }
        Ok(Self { data: *result })
    }

    /// Cosine similarity between two float hypervectors.
    fn cosine_similarity(&self, other: &Self) -> f32 {
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;
        for i in 0..10240 {
            dot += self.data[i] * other.data[i];
            norm_a += self.data[i] * self.data[i];
            norm_b += other.data[i] * other.data[i];
        }
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }

    /// Hamming distance is not standard for floats; we use sign-based Hamming if needed,
    /// but for Hypervector trait consistency on floats we could return an error or
    /// implement it via thresholding. ADR-0075 implies HVec10240 is f32.
    /// Here we implement Hamming based on sign (quantized Hamming).
    fn hamming_distance(&self, other: &Self) -> u32 {
        let mut dist = 0u32;
        for i in 0..10240 {
            let sign_a = self.data[i] >= 0.0;
            let sign_b = other.data[i] >= 0.0;
            if sign_a != sign_b {
                dist += 1;
            }
        }
        dist
    }

    /// Permute the hypervector (cyclic rotation)
    fn permute(&self, shift: usize) -> Self {
        let mut result = [0.0f32; 10240];
        let shift = shift % 10240;
        if shift == 0 {
            return *self;
        }
        let (left, right) = self.data.split_at(10240 - shift);
        result[..shift].copy_from_slice(right);
        result[shift..].copy_from_slice(left);
        Self { data: result }
    }

    /// Serialize to bytes (40 KB)
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(10240 * 4);
        for &val in &self.data {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 40960 {
            // If we receive 1280 bytes, it might be a legacy binary vector being loaded into HVec10240.
            // But HVec10240 is now f32. We should probably handle this in Persistence or error out.
            return Err(crate::error::MemoryError::InvalidDimension {
                expected: 40960,
                actual: bytes.len(),
            });
        }

        let mut data = [0.0f32; 10240];
        for i in 0..10240 {
            let mut val_bytes = [0u8; 4];
            val_bytes.copy_from_slice(&bytes[i * 4..(i + 1) * 4]);
            data[i] = f32::from_le_bytes(val_bytes);
        }

        Ok(Self { data })
    }
}

// Re-export BundleAccumulator from bundle module
pub use crate::bundle::BundleAccumulator;

impl serde::Serialize for HVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD;
            let bytes = self.to_bytes();
            let b64 = STANDARD.encode(&bytes);
            serializer.serialize_str(&b64)
        } else {
            let bytes = self.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> serde::Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct HVecVisitor;
        impl<'de> serde::de::Visitor<'de> for HVecVisitor {
            type Value = HVec10240;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a base64 string or byte array for HVec10240")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                use base64::Engine;
                use base64::engine::general_purpose::STANDARD;
                let bytes = STANDARD.decode(v).map_err(E::custom)?;
                HVec10240::from_bytes(&bytes).map_err(E::custom)
            }
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> std::result::Result<Self::Value, E> {
                HVec10240::from_bytes(v).map_err(E::custom)
            }
        }
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(HVecVisitor)
        } else {
            let bytes = <Vec<u8>>::deserialize(deserializer)?;
            Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hvec_creation() {
        let vec = HVec10240::zero();
        assert_eq!(vec.data.iter().sum::<f32>(), 0.0);
    }

    #[test]
    fn test_random_generation() {
        let vec1 = HVec10240::random();
        let vec2 = HVec10240::random();
        assert_ne!(vec1.data, vec2.data);
    }

    #[test]
    fn test_self_similarity() {
        let vec = HVec10240::random();
        let similarity = vec.cosine_similarity(&vec);
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_serialization() {
        let v = HVec10240::random();
        let bytes = v.to_bytes();
        assert_eq!(bytes.len(), 40960);
        let v2 = HVec10240::from_bytes(&bytes).unwrap();
        assert_eq!(v.data, v2.data);
    }

    #[test]
    fn test_permute() {
        let v = HVec10240::random();
        assert_eq!(v, v.permute(0));
        let s = v.permute(1);
        assert_eq!(s.data[0], v.data[10239]);
        assert_eq!(s.data[1], v.data[0]);
    }
}
