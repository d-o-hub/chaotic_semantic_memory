//! Binary Hypervectors (ADR-0075)
//!
//! 10240-bit hypervectors packed into 160 x u64 words.
//! Provides 32x compression compared to f32 hypervectors.

use crate::error::{MemoryError, Result};
use crate::hyperdim::{HVec10240, Hypervector};
use rand::RngExt;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct BHVec10240 {
    pub bits: [u64; 160],
}

impl Serialize for BHVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
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

struct BHVecVisitor;

impl<'de> Visitor<'de> for BHVecVisitor {
    type Value = BHVec10240;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string or byte array")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let bytes = STANDARD.decode(v).map_err(de::Error::custom)?;
        BHVec10240::from_bytes(&bytes).map_err(de::Error::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        BHVec10240::from_bytes(v).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for BHVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(BHVecVisitor)
        } else {
            let bytes = <Vec<u8>>::deserialize(deserializer)?;
            Self::from_bytes(&bytes).map_err(de::Error::custom)
        }
    }
}

impl Hypervector for BHVec10240 {
    const DIMENSION: usize = 10240;
    const FORMAT_NAME: &'static str = "binary";

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
        Ok(Self::bundle(vectors))
    }

    fn bind(&self, other: &Self) -> Self {
        self.xor(other)
    }

    fn cosine_similarity(&self, other: &Self) -> f32 {
        self.cosine_similarity(other)
    }

    fn hamming_distance(&self, other: &Self) -> u32 {
        self.hamming(other)
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

impl BHVec10240 {
    pub const DIMENSION: usize = 10240;
    pub const WORDS: usize = 160;

    /// Create a new hypervector with all zeros
    pub const fn zero() -> Self {
        Self { bits: [0u64; 160] }
    }

    /// Create a random hypervector
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let mut bits = [0u64; 160];
        rng.fill(&mut bits);
        Self { bits }
    }

    /// Create a deterministic random hypervector from a seed
    pub fn new_seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut bits = [0u64; 160];
        rng.fill(&mut bits);
        Self { bits }
    }

    /// Convert HVec10240 (bit-packed u128) to BHVec10240 (bit-packed u64)
    /// This is just a layout conversion.
    pub fn from_hvec(v: &HVec10240) -> Self {
        let mut bits = [0u64; 160];
        for i in 0..80 {
            bits[i * 2] = v.data[i] as u64;
            bits[i * 2 + 1] = (v.data[i] >> 64) as u64;
        }
        Self { bits }
    }

    /// Convert BHVec10240 (bit-packed u64) to HVec10240 (bit-packed u128)
    pub fn to_hvec(&self) -> HVec10240 {
        let mut data = [0u128; 80];
        for i in 0..80 {
            data[i] = (self.bits[i * 2] as u128) | ((self.bits[i * 2 + 1] as u128) << 64);
        }
        HVec10240 { data }
    }

    /// XOR binding
    pub fn xor(&self, other: &Self) -> Self {
        let mut result = [0u64; 160];
        for i in 0..160 {
            result[i] = self.bits[i] ^ other.bits[i];
        }
        Self { bits: result }
    }

    /// Hamming distance (popcount of XOR)
    pub fn hamming(&self, other: &Self) -> u32 {
        let mut dist = 0u32;
        for i in 0..160 {
            dist += (self.bits[i] ^ other.bits[i]).count_ones();
        }
        dist
    }

    /// Cosine similarity (approximated for binary as 1 - Hamming/Dimension/2)
    /// Similarity = 1.0 - (HammingDistance / 5120.0)
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let dist = self.hamming(other);
        1.0 - (dist as f32 / 5120.0)
    }

    /// Bundle multiple hypervectors using majority rule
    pub fn bundle(vectors: &[&Self]) -> Self {
        if vectors.is_empty() {
            return Self::zero();
        }
        if vectors.len() == 1 {
            return *vectors[0];
        }

        let mut result = [0u64; 160];
        let threshold = vectors.len() / 2;

        // Simple scalar implementation for now
        // For production, this should be optimized with bit-sliced addition
        for i in 0..Self::DIMENSION {
            let mut count = 0;
            let word_idx = i / 64;
            let bit_idx = i % 64;
            let mask = 1u64 << bit_idx;

            for v in vectors {
                if (v.bits[word_idx] & mask) != 0 {
                    count += 1;
                }
            }

            if count > threshold {
                result[word_idx] |= mask;
            }
        }

        Self { bits: result }
    }

    /// Cyclic permutation (shift)
    pub fn permute(&self, shift: usize) -> Self {
        let mut result = [0u64; 160];
        let bit_shift = shift % 64;
        let word_shift = (shift / 64) % 160;

        for i in 0..160 {
            let src_idx = (i + 160 - word_shift) % 160;
            let next_idx = (src_idx + 159) % 160;

            let val = if bit_shift == 0 {
                self.bits[src_idx]
            } else {
                (self.bits[src_idx] << bit_shift) | (self.bits[next_idx] >> (64 - bit_shift))
            };
            result[i] = val;
        }

        Self { bits: result }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1280);
        for word in &self.bits {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }
        let mut bits = [0u64; 160];
        for i in 0..160 {
            let mut word_bytes = [0u8; 8];
            word_bytes.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            bits[i] = u64::from_le_bytes(word_bytes);
        }
        Ok(Self { bits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bhvec_random() {
        let v1 = BHVec10240::random();
        let v2 = BHVec10240::random();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_bhvec_xor_hamming() {
        let v1 = BHVec10240::random();
        let v2 = BHVec10240::random();
        let bound = v1.xor(&v2);
        let dist = v1.hamming(&v2);
        assert_eq!(bound.bits.iter().map(|w| w.count_ones()).sum::<u32>(), dist);
    }

    #[test]
    fn test_bhvec_permute() {
        let v1 = BHVec10240::random();
        let v2 = v1.permute(1);
        assert_ne!(v1, v2);
        let v3 = v2.permute(BHVec10240::DIMENSION - 1);
        assert_eq!(v1, v3);
    }

    #[test]
    fn test_bhvec_roundtrip_hvec() {
        let h1 = HVec10240::random();
        let bh1 = BHVec10240::from_hvec(&h1);
        let h2 = bh1.to_hvec();
        assert_eq!(h1, h2);
    }
}
