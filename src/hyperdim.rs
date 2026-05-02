//! Hyperdimensional computing primitives
//!
//! Implements 10240-bit hypervectors using `[u128; 80]`.

// Casts are intentional for HDC dimension math (10240-bit operations)
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use rand::RngExt;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use crate::error::Result;

pub use crate::hyperdim_batch::batch_cosine_similarity;

// Import SIMD functions from extension module
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
use crate::hyperdim_simd::bind_simd_avx2;
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
use crate::hyperdim_simd::bind_simd_neon;
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
use crate::hyperdim_simd::bind_simd_x86;
use crate::hyperdim_simd::hamming_distance_optimized;

/// 10240-bit hypervector (80 x 128-bit words)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct HVec10240 {
    pub(crate) data: [u128; 80],
}

impl HVec10240 {
    pub const DIMENSION: usize = 10240;
    pub const WORDS: usize = 80;

    /// Create a new hypervector with all zeros
    pub const fn zero() -> Self {
        Self { data: [0u128; 80] }
    }

    /// Create a random hypervector (each bit has 50% probability)
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let mut data = [0u128; 80];
        for word in &mut data {
            *word = rng.random();
        }
        Self { data }
    }

    /// Create a deterministic random hypervector from a seed.
    ///
    /// Uses `rand::rngs::StdRng` for reproducibility across runs.
    pub fn new_seeded(seed: u64) -> Self {
        use rand::rngs::StdRng;
        use rand::{RngExt, SeedableRng};

        let mut rng = StdRng::seed_from_u64(seed);
        let mut data = [0u128; 80];
        for word in &mut data {
            *word = rng.random();
        }
        Self { data }
    }

    /// Create a random sparse hypervector with given density
    pub fn sparse(density: f32) -> Self {
        let mut rng = rand::rng();
        let mut data = [0u128; 80];
        let bits_to_set = (Self::DIMENSION as f32 * density) as usize;

        for _ in 0..bits_to_set {
            let pos = rng.random_range(0..Self::DIMENSION);
            let word = pos / 128;
            let bit = pos % 128;
            data[word] |= 1u128 << bit;
        }

        Self { data }
    }

    /// Bundle (sum) multiple hypervectors using bit-sliced addition.
    ///
    /// This implementation is optimized for performance and memory efficiency:
    /// 1. It uses word-parallel bit-sliced addition to count set bits across vectors.
    /// 2. It eliminates the large heap-allocated counter array and bit-by-bit loops.
    /// 3. It parallelizes over hypervector words rather than over vectors to minimize
    ///    memory traffic and synchronization overhead.
    pub fn bundle(vectors: &[Self]) -> Result<Self> {
        let num_vectors = vectors.len();
        match num_vectors {
            0 => Ok(Self::zero()),
            // Fast-path: single vector bundle is the vector itself
            1 => Ok(vectors[0]),
            // Fast-path: majority rule for 2 vectors requires both bits set (AND)
            2 => {
                let mut data = [0u128; 80];
                for i in 0..80 {
                    data[i] = vectors[0].data[i] & vectors[1].data[i];
                }
                Ok(Self { data })
            }
            _ => {
                // Threshold: strictly greater than half
                let threshold = num_vectors / 2 + 1;
                // Number of bit-planes needed to represent a sum up to num_vectors
                let num_planes = (usize::BITS - num_vectors.leading_zeros()) as usize;

                let mut data = [0u128; 80];

                // Optimization: Use sequential path for small vector sets to avoid Rayon overhead.
                // Task scheduling for 80 words is only profitable when the work per word
                // (proportional to num_vectors) outweighs the synchronization cost.
                #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
                let use_parallel = num_vectors >= 32;
                #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
                let use_parallel = false;

                if use_parallel {
                    #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
                    {
                        data.par_iter_mut().enumerate().for_each(|(i, word)| {
                            let mut planes = [0u128; 32];
                            for v in vectors {
                                let mut carry = v.data[i];
                                for plane in planes.iter_mut().take(num_planes) {
                                    let next_carry = *plane & carry;
                                    *plane ^= carry;
                                    carry = next_carry;
                                    if carry == 0 {
                                        break;
                                    }
                                }
                            }

                            let mut current_eq = !0u128;
                            let mut current_gt = 0u128;
                            for p in (0..num_planes).rev() {
                                let bit = (threshold >> p) & 1;
                                if bit == 1 {
                                    current_eq &= planes[p];
                                } else {
                                    current_gt |= current_eq & planes[p];
                                    current_eq &= !planes[p];
                                }
                            }
                            *word = current_gt | current_eq;
                        });
                    }
                } else {
                    for i in 0..80 {
                        let mut planes = [0u128; 32];
                        for v in vectors {
                            let mut carry = v.data[i];
                            for plane in planes.iter_mut().take(num_planes) {
                                let next_carry = *plane & carry;
                                *plane ^= carry;
                                carry = next_carry;
                                if carry == 0 {
                                    break;
                                }
                            }
                        }

                        let mut current_eq = !0u128;
                        let mut current_gt = 0u128;
                        for p in (0..num_planes).rev() {
                            let bit = (threshold >> p) & 1;
                            if bit == 1 {
                                current_eq &= planes[p];
                            } else {
                                current_gt |= current_eq & planes[p];
                                current_eq &= !planes[p];
                            }
                        }
                        data[i] = current_gt | current_eq;
                    }
                }

                Ok(Self { data })
            }
        }
    }

    /// XOR binding of two hypervectors.
    ///
    /// Dispatches to optimized SIMD paths based on platform:
    /// - x86_64: AVX2 (runtime detection) or SSE fallback
    /// - aarch64: NEON
    /// - Other: scalar XOR
    pub fn bind(&self, other: &Self) -> Self {
        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        {
            // Runtime dispatch: AVX2 if available, else SSE fallback
            if is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 feature detected at runtime.
                Self {
                    data: unsafe { bind_simd_avx2(&self.data, &other.data) },
                }
            } else {
                Self {
                    data: bind_simd_x86(&self.data, &other.data),
                }
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86"))]
        {
            Self {
                data: bind_simd_x86(&self.data, &other.data),
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
        {
            // SAFETY: bind_simd_neon requires unsafe due to NEON intrinsics.
            // The function is marked #[target_feature(enable = "neon")] which
            // is always available on aarch64, making this call safe.
            Self {
                data: unsafe { bind_simd_neon(&self.data, &other.data) },
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut result = [0u128; 80];
            for i in 0..80 {
                result[i] = self.data[i] ^ other.data[i];
            }
            Self { data: result }
        }

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64"))
        ))]
        {
            let mut result = [0u128; 80];
            for i in 0..80 {
                result[i] = self.data[i] ^ other.data[i];
            }
            Self { data: result }
        }
    }

    /// Cosine similarity between two hypervectors.
    ///
    /// Calculated as `1.0 - (HammingDistance / 5120.0)` for 10240-bit vectors.
    /// This implementation is unified across all platforms and uses an unrolled
    /// GPR popcount loop for maximum performance.
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let distance = hamming_distance_optimized(&self.data, &other.data);
        // Similarity = (Matches - Mismatches) / Dimension
        // Similarity = (Dimension - 2 * HammingDistance) / Dimension
        // Similarity = 1.0 - (2.0 * HammingDistance / 10240.0) = 1.0 - (HammingDistance / 5120.0)
        1.0 - (distance as f32 / 5120.0)
    }

    /// Hamming distance
    #[must_use]
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        hamming_distance_optimized(&self.data, &other.data)
    }

    /// Permute the hypervector (cyclic rotation)
    ///
    /// Optimized implementation that eliminates modulo operations and branches
    /// from the hot loop by splitting the rotation into two contiguous segments.
    #[allow(clippy::needless_range_loop)]
    pub fn permute(&self, shift: usize) -> Self {
        let mut result = [0u128; 80];
        let bit_shift = shift % 128;
        let word_shift = (shift / 128) % 80;

        // Optimized path for word-aligned rotations
        if bit_shift == 0 {
            let (left, right) = self.data.split_at(word_shift);
            result[..80 - word_shift].copy_from_slice(right);
            result[80 - word_shift..].copy_from_slice(left);
            return Self { data: result };
        }

        let inv_bit_shift = 128 - bit_shift;

        // Split cyclic rotation into two segments to eliminate modulo in the loop
        // Segment 1: src1 from word_shift to 78, src2 from word_shift + 1 to 79
        let limit = 79 - word_shift;
        for i in 0..limit {
            let src1 = i + word_shift;
            let src2 = src1 + 1;
            result[i] = (self.data[src1] << bit_shift) | (self.data[src2] >> inv_bit_shift);
        }

        // Handle the wrap-around word at the boundary of segment 1 and 2
        // result[79 - word_shift] uses data[79] and data[0]
        result[limit] = (self.data[79] << bit_shift) | (self.data[0] >> inv_bit_shift);

        // Segment 2: src1 from 0 to word_shift - 1, src2 from 1 to word_shift
        for i in limit + 1..80 {
            let src1 = i + word_shift - 80;
            let src2 = src1 + 1;
            result[i] = (self.data[src1] << bit_shift) | (self.data[src2] >> inv_bit_shift);
        }

        Self { data: result }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1280);
        for word in &self.data {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(crate::error::MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }

        let mut data = [0u128; 80];
        for i in 0..80 {
            let mut word_bytes = [0u8; 16];
            word_bytes.copy_from_slice(&bytes[i * 16..(i + 1) * 16]);
            data[i] = u128::from_le_bytes(word_bytes);
        }

        Ok(Self { data })
    }
}

impl Serialize for HVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Use base64 for JSON and other human-readable formats
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD;
            let bytes = self.to_bytes();
            let b64 = STANDARD.encode(&bytes);
            serializer.serialize_str(&b64)
        } else {
            // Use fixed-size array for binary formats (bincode compatible)
            let bytes = self.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

struct HVecVisitor;

impl<'de> Visitor<'de> for HVecVisitor {
    type Value = HVec10240;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string or byte array of length 1280")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let bytes = STANDARD.decode(v).map_err(de::Error::custom)?;
        HVec10240::from_bytes(&bytes).map_err(de::Error::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        HVec10240::from_bytes(v).map_err(de::Error::custom)
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // Handle JSON array of numbers (legacy format)
        let mut bytes = Vec::with_capacity(1280);
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        if bytes.len() != 1280 {
            return Err(de::Error::custom(format!(
                "expected 1280 bytes, got {}",
                bytes.len()
            )));
        }
        HVec10240::from_bytes(&bytes).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use deserialize_any to handle both string (base64) and bytes formats
        deserializer.deserialize_any(HVecVisitor)
    }
}

// Re-export BundleAccumulator from bundle module
pub use crate::bundle::BundleAccumulator;

include!("hyperdim_tests.rs");
