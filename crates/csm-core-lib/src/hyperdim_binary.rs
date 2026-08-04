//! Binary Hypervectors (ADR-0075)
//!
//! 10240-bit hypervectors packed into 160 x u64 words.
//! Provides 32x compression compared to f32 hypervectors.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::error::{MemoryError, Result};
use crate::hyperdim::{HVec10240, Hypervector};
use crate::hyperdim_ops::bundle_word_u64;
use rand::RngExt;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(align(16))]
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

    /// Hamming distance (popcount of XOR).
    ///
    /// Algorithmic Optimization: Since BHVec10240 has #[repr(align(16))], we can safely
    /// cast &[u64; 160] to &[u128; 80] without any alignment-related panics. This bypasses
    /// the layout conversion to HVec10240 (to_hvec()) and temporary stack allocation,
    /// and allows direct dispatch to SIMD-optimized Hamming distance helpers.
    pub fn hamming(&self, other: &Self) -> u32 {
        // SAFETY: BHVec10240 has #[repr(align(16))], ensuring that `self.bits` (and `other.bits`)
        // are aligned to at least 16 bytes. Both arrays are exactly 1280 bytes.
        // Therefore, casting `&[u64; 160]` to `&[u128; 80]` is safe, correct, and zero-allocation.
        let lhs_u128: &[u128; 80] = unsafe { &*(self.bits.as_ptr() as *const [u128; 80]) };
        let rhs_u128: &[u128; 80] = unsafe { &*(other.bits.as_ptr() as *const [u128; 80]) };

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        {
            if std::is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 feature detected at runtime. Both lhs and rhs are 16-byte aligned.
                unsafe { crate::hyperdim_simd::hamming_distance_simd_avx2(lhs_u128, rhs_u128) }
            } else {
                crate::hyperdim_simd::hamming_distance_optimized(lhs_u128, rhs_u128)
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
        {
            // SAFETY: aarch64 always has NEON. Both lhs and rhs are 16-byte aligned.
            unsafe { crate::hyperdim_simd::hamming_distance_simd_neon(lhs_u128, rhs_u128) }
        }

        #[cfg(any(
            target_arch = "wasm32",
            not(any(target_arch = "x86_64", target_arch = "aarch64"))
        ))]
        {
            crate::hyperdim_simd::hamming_distance_optimized(lhs_u128, rhs_u128)
        }
    }

    /// Cosine similarity (approximated for binary as 1 - Hamming/Dimension/2)
    /// Similarity = 1.0 - (HammingDistance / 5120.0)
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let dist = self.hamming(other);
        1.0 - (dist as f32 / 5120.0)
    }

    /// Bundle multiple hypervectors using bit-sliced addition.
    ///
    /// Algorithmic Optimization: Replaces the O(D * N) bit-by-bit loop with a transposed
    /// bit-sliced addition approach. By processing all 160 words of each vector contiguously,
    /// we eliminate 160x redundant memory loads, achieving a massive locality speedup
    /// with 100% safe Rust.
    ///
    /// Majority rule: a bit is set when `count >= N/2 + 1` (equivalent to `count > N/2`).
    /// N=2 is a fast path (bitwise AND). Accumulation capacity matches HVec via
    /// [`crate::hyperdim_ops::BUNDLE_MAX_PLANES`] (64 planes).
    pub fn bundle(vectors: &[&Self]) -> Self {
        let num_vectors = vectors.len();
        if num_vectors == 0 {
            return Self::zero();
        }
        if num_vectors == 1 {
            return *vectors[0];
        }
        // N=2 majority is bitwise AND (threshold = 2). Match HVec fast path.
        if num_vectors == 2 {
            let mut bits = [0u64; 160];
            for i in 0..160 {
                bits[i] = vectors[0].bits[i] & vectors[1].bits[i];
            }
            return Self { bits };
        }

        let threshold = num_vectors / 2 + 1;
        let num_planes = (usize::BITS - num_vectors.leading_zeros()) as usize;

        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        if num_vectors >= 256 {
            let mut bits = [0u64; 160];
            bits.par_iter_mut().enumerate().for_each(|(i, word)| {
                let mut planes = [0u64; 64];
                for v in vectors {
                    let mut carry = v.bits[i];
                    for p in 0..num_planes {
                        let next_carry = planes[p] & carry;
                        planes[p] ^= carry;
                        carry = next_carry;
                        if carry == 0 {
                            break;
                        }
                    }
                }
                let (mut current_eq, mut current_gt) = (!0u64, 0u64);
                for p in (0..num_planes).rev() {
                    if ((threshold >> p) & 1) == 1 {
                        current_eq &= planes[p];
                    } else {
                        current_gt |= current_eq & planes[p];
                        current_eq &= !planes[p];
                    }
                }
                *word = current_gt | current_eq;
            });
            return Self { bits };
        }

        // Cache-friendly transposed bit-sliced addition
        let mut planes = vec![[0u64; 160]; num_planes];
        for v in vectors {
            for i in 0..160 {
                let mut carry = v.bits[i];
                for p in 0..num_planes {
                    let next_carry = planes[p][i] & carry;
                    planes[p][i] ^= carry;
                    carry = next_carry;
                    if carry == 0 {
                        break;
                    }
                }
            }
        }

        let mut bits = [0u64; 160];
        for i in 0..160 {
            let (mut current_eq, mut current_gt) = (!0u64, 0u64);
            for p in (0..num_planes).rev() {
                if ((threshold >> p) & 1) == 1 {
                    current_eq &= planes[p][i];
                } else {
                    current_gt |= current_eq & planes[p][i];
                    current_eq &= !planes[p][i];
                }
            }
            bits[i] = current_gt | current_eq;
        }

        Self { bits }
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
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_bhvec_random() {
        assert_ne!(BHVec10240::random(), BHVec10240::random());
    }

    #[test]
    fn test_bhvec_xor_hamming() {
        let (v1, v2) = (BHVec10240::random(), BHVec10240::random());
        assert_eq!(v1.xor(&v2).bits.iter().map(|w| w.count_ones()).sum::<u32>(), v1.hamming(&v2));
    }

    #[test]
    fn test_bhvec_hamming_edge_cases() {
        let (zero, ones) = (BHVec10240::zero(), BHVec10240 { bits: [!0u64; 160] });
        let mut edge_bits = [0u64; 160];
        edge_bits[0] = 1;
        edge_bits[159] = 1 << 63;
        let edges = BHVec10240 { bits: edge_bits };
        assert_eq!(zero.hamming(&zero), 0);
        assert_eq!(zero.hamming(&ones), 10240);
        assert_eq!(zero.hamming(&edges), 2);
    }

    #[test]
    fn test_bhvec_hamming_matches_scalar_oracle() {
        let (lhs, rhs) = (BHVec10240::new_seeded(42), BHVec10240::new_seeded(84));
        let expected = lhs.bits.iter().zip(&rhs.bits).map(|(l, r)| (l ^ r).count_ones()).sum::<u32>();
        assert_eq!(lhs.hamming(&rhs), expected);
    }

    #[test]
    fn test_bhvec_permute() {
        let v1 = BHVec10240::random();
        assert_ne!(v1, v1.permute(1));
        assert_eq!(v1, v1.permute(1).permute(BHVec10240::DIMENSION - 1));
    }

    #[test]
    fn test_bhvec_roundtrip_hvec() {
        let h1 = HVec10240::random();
        assert_eq!(h1, BHVec10240::from_hvec(&h1).to_hvec());
    }

    fn naive_bundle_majority(vectors: &[&BHVec10240]) -> BHVec10240 {
        let n = vectors.len();
        if n == 0 { return BHVec10240::zero(); }
        if n == 1 { return *vectors[0]; }
        let threshold = n / 2 + 1;
        let mut bits = [0u64; 160];
        for w in 0..160 {
            for b in 0..64 {
                let mask = 1u64 << b;
                let count = vectors.iter().filter(|v| (v.bits[w] & mask) != 0).count();
                if count >= threshold { bits[w] |= mask; }
            }
        }
        BHVec10240 { bits }
    }

    #[test]
    fn test_bhvec_bundle_empty_and_single() {
        assert_eq!(BHVec10240::bundle(&[]), BHVec10240::zero());
        let v = BHVec10240::new_seeded(42);
        assert_eq!(BHVec10240::bundle(&[&v]), v);
    }

    #[test]
    fn test_bhvec_bundle_n2_is_and() {
        let (v1, v2) = (BHVec10240::new_seeded(1), BHVec10240::new_seeded(2));
        let bundled = BHVec10240::bundle(&[&v1, &v2]);
        for i in 0..160 {
            assert_eq!(bundled.bits[i], v1.bits[i] & v2.bits[i]);
        }
    }

    #[test]
    fn test_bhvec_bundle_threshold_consistency() {
        for n in [2, 3, 4, 10, 255, 256, 1000] {
            let vectors: Vec<BHVec10240> = (0..n).map(|i| BHVec10240::new_seeded(i as u64)).collect();
            let refs: Vec<&BHVec10240> = vectors.iter().collect();
            assert_eq!(BHVec10240::bundle(&refs).bits, naive_bundle_majority(&refs).bits);
        }
    }
}
