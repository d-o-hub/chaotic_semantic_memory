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
    /// Dispatches directly over the packed `[u64; 160]` words — AVX2 on
    /// x86_64, NEON on aarch64, and an unrolled scalar popcount loop
    /// elsewhere (including wasm32) — avoiding conversion to temporary
    /// [`HVec10240`] values entirely.
    pub fn hamming(&self, other: &Self) -> u32 {
        crate::hyperdim_simd::hamming_distance_u64(&self.bits, &other.bits)
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
            #[cfg(target_arch = "x86_64")]
            if is_x86_feature_detected!("avx2") {
                let mut bits = [0u64; 160];
                bits.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                    // SAFETY: AVX2 detected at runtime. Pointers within bounds.
                    let res = unsafe {
                        crate::hyperdim_simd_bundle::bundle_block_avx2_single_u64(
                            vectors,
                            i * 4,
                            threshold,
                            num_planes,
                        )
                    };
                    // SAFETY: chunk length is 4 (32 bytes), matching AVX2 256-bit block size.
                    unsafe {
                        std::arch::x86_64::_mm256_storeu_si256(chunk.as_mut_ptr().cast(), res);
                    }
                });
                return Self { bits };
            }

            #[cfg(target_arch = "aarch64")]
            {
                let mut bits = [0u64; 160];
                bits.par_chunks_mut(2).enumerate().for_each(|(i, chunk)| {
                    // SAFETY: NEON always available on aarch64. Pointers within bounds.
                    let res = unsafe {
                        crate::hyperdim_simd_bundle::bundle_block_neon_single_u64(
                            vectors,
                            i * 2,
                            threshold,
                            num_planes,
                        )
                    };
                    // SAFETY: chunk length is 2 (16 bytes), matching NEON 128-bit block size.
                    unsafe {
                        std::arch::aarch64::vst1q_u8(chunk.as_mut_ptr().cast(), res);
                    }
                });
                return Self { bits };
            }

            #[cfg(not(target_arch = "aarch64"))]
            {
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
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 is detected at runtime.
            return Self {
                bits: unsafe {
                    crate::hyperdim_simd_bundle::bundle_block_avx2_u64(
                        vectors, threshold, num_planes,
                    )
                },
            };
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
        {
            // SAFETY: NEON is always available on aarch64.
            return Self {
                bits: unsafe {
                    crate::hyperdim_simd_bundle::bundle_block_neon_u64(
                        vectors, threshold, num_planes,
                    )
                },
            };
        }

        #[cfg(not(all(not(target_arch = "wasm32"), target_arch = "aarch64")))]
        {
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
    }

    /// Cyclic permutation (shift)
    ///
    /// Optimized implementation that eliminates modulo operations and branches
    /// from the hot loop by splitting the rotation into three contiguous segments.
    pub fn permute(&self, shift: usize) -> Self {
        let mut result = [0u64; 160];
        let bit_shift = shift % 64;
        let word_shift = (shift / 64) % 160;

        // Optimized path for word-aligned rotations
        if bit_shift == 0 {
            if word_shift == 0 {
                return *self;
            }
            let (left, right) = self.bits.split_at(160 - word_shift);
            result[..word_shift].copy_from_slice(right);
            result[word_shift..].copy_from_slice(left);
            return Self { bits: result };
        }

        let inv_bit_shift = 64 - bit_shift;

        // Segment 1: i < word_shift
        // src_idx goes from 160 - word_shift to 159.
        // next_idx is always src_idx - 1.
        for i in 0..word_shift {
            let src_idx = i + 160 - word_shift;
            let next_idx = src_idx - 1;
            result[i] = (self.bits[src_idx] << bit_shift) | (self.bits[next_idx] >> inv_bit_shift);
        }

        // Segment 2: i = word_shift
        // src_idx = 0, next_idx = 159.
        if word_shift < 160 {
            result[word_shift] = (self.bits[0] << bit_shift) | (self.bits[159] >> inv_bit_shift);
        }

        // Segment 3: i > word_shift
        // src_idx goes from 1 to 159 - word_shift.
        // next_idx is always src_idx - 1.
        for i in word_shift + 1..160 {
            let src_idx = i - word_shift;
            let next_idx = src_idx - 1;
            result[i] = (self.bits[src_idx] << bit_shift) | (self.bits[next_idx] >> inv_bit_shift);
        }

        Self { bits: result }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1280);
        #[cfg(target_endian = "little")]
        {
            // Performance Optimization: [u64; 160] is bit-compatible with [u8; 1280]
            // on little-endian platforms. Using extend_from_slice with a casted
            // byte reference avoids 160 bounds checks and word-by-word serialization.
            // SAFETY:
            // 1. Pointer Validity: `self.bits` is an initialized `[u64; 160]` array, which occupies
            //    exactly 1,280 contiguous bytes of memory.
            // 2. Alignment: `u64` has a stricter alignment constraint (8 bytes) than `u8` (1 byte).
            //    Casting a stricter aligned pointer (`*const u64`) to a weaker aligned pointer (`*const u8`)
            //    is always safe and does not cause alignment violations.
            // 3. Lifetime: The returned reference is bound to the lifetime of `self`, and we copy
            //    its contents immediately into `bytes` before the reference is discarded.
            let data_bytes: &[u8; 1280] = unsafe { &*(self.bits.as_ptr() as *const [u8; 1280]) };
            bytes.extend_from_slice(data_bytes);
        }
        #[cfg(not(target_endian = "little"))]
        {
            for word in &self.bits {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
        bytes
    }

    /// Deserialize from bytes
    #[allow(clippy::missing_const_for_fn)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }
        #[allow(unused_mut)]
        let mut bits = [0u64; 160];
        #[cfg(target_endian = "little")]
        {
            // Performance Optimization: Direct memcpy for little-endian platforms.
            // Avoids 160 loop iterations and multiple bounds checks per word.
            // SAFETY: bytes length is verified to be 1280. [u64; 160] is bit-compatible
            // with [u8; 1280] on little-endian. Pointers are valid.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), bits.as_mut_ptr() as *mut u8, 1280);
            }
        }
        #[cfg(not(target_endian = "little"))]
        {
            for i in 0..160 {
                let mut word_bytes = [0u8; 8];
                word_bytes.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
                bits[i] = u64::from_le_bytes(word_bytes);
            }
        }
        Ok(Self { bits })
    }
}
