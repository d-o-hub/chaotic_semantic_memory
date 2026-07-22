//! Hyperdimensional computing primitives
//!
//! Implements 10240-bit hypervectors using `[u128; 80]`.

// Casts are intentional for HDC dimension math (10240-bit operations)
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use rand::RngExt;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use crate::error::Result;

pub use crate::hyperdim_batch::batch_cosine_similarity;
pub use crate::hyperdim_binary::BHVec10240;
pub use crate::hyperdim_ops::{Hypervector, bundle_word_scalar};

// Import SIMD functions from extension module
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
use crate::hyperdim_simd::{and_simd_avx2, bind_simd_avx2, hamming_distance_simd_avx2};
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
use crate::hyperdim_simd::{and_simd_neon, bind_simd_neon, hamming_distance_simd_neon};
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
use crate::hyperdim_simd::{and_simd_x86, bind_simd_x86};

#[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
use crate::hyperdim_simd_bundle::bundle_block_avx2;

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
use crate::hyperdim_simd_bundle::bundle_block_neon;

/// 10240-bit hypervector (80 x 128-bit words)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub struct HVec10240 {
    pub data: [u128; 80],
}

impl HVec10240 {
    pub const DIMENSION: usize = 10240;
    pub const WORDS: usize = 80;

    /// Create a new hypervector with all zeros
    pub const fn zero() -> Self {
        Self { data: [0u128; 80] }
    }

    /// Create a random hypervector (each bit has 50% probability)
    ///
    /// Performance Optimization: Uses `rng.fill()` for bulk data generation, reducing
    /// per-word overhead and allowing the RNG to use vectorized memory-filling paths.
    /// Expected speedup: ~15% for random generation.
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let mut data = [0u128; 80];
        rng.fill(&mut data);
        Self { data }
    }

    /// Create a deterministic random hypervector from a seed.
    ///
    /// Uses `rand::rngs::StdRng` for reproducibility across runs.
    pub fn new_seeded(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut data = [0u128; 80];
        rng.fill(&mut data);
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

    /// Set a specific bit in the hypervector.
    ///
    /// # Panics
    /// Panics if `pos >= DIMENSION` (10240).
    pub fn set_bit(&mut self, pos: usize) {
        assert!(
            pos < Self::DIMENSION,
            "bit position {pos} out of range (max {})",
            Self::DIMENSION
        );
        let word = pos / 128;
        let bit = pos % 128;
        self.data[word] |= 1u128 << bit;
    }

    /// Bundle (sum) multiple hypervectors using bit-sliced addition.
    ///
    /// This implementation is optimized for performance and memory efficiency:
    /// 1. It uses word-parallel bit-sliced addition to count set bits across vectors.
    /// 2. It eliminates the large heap-allocated counter array and bit-by-bit loops.
    /// 3. It parallelizes over hypervector words rather than over vectors to minimize
    ///    memory traffic and synchronization overhead.
    #[allow(clippy::needless_range_loop)]
    pub fn bundle(vectors: &[Self]) -> Result<Self> {
        let num_vectors = vectors.len();
        if num_vectors == 0 {
            return Ok(Self::zero());
        }
        if num_vectors == 1 {
            return Ok(vectors[0]);
        }
        if num_vectors == 2 {
            #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
            {
                if is_x86_feature_detected!("avx2") {
                    // SAFETY: AVX2 is detected at runtime.
                    return Ok(Self {
                        data: unsafe { and_simd_avx2(&vectors[0].data, &vectors[1].data) },
                    });
                } else {
                    return Ok(Self {
                        data: and_simd_x86(&vectors[0].data, &vectors[1].data),
                    });
                }
            }

            #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86"))]
            {
                return Ok(Self {
                    data: and_simd_x86(&vectors[0].data, &vectors[1].data),
                });
            }

            #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
            {
                // SAFETY: NEON is always available on aarch64.
                return Ok(Self {
                    data: unsafe { and_simd_neon(&vectors[0].data, &vectors[1].data) },
                });
            }

            #[cfg(any(
                target_arch = "wasm32",
                all(
                    not(target_arch = "wasm32"),
                    not(any(
                        target_arch = "x86_64",
                        target_arch = "x86",
                        target_arch = "aarch64"
                    ))
                )
            ))]
            {
                let mut res = Self::zero();
                for i in 0..80 {
                    res.data[i] = vectors[0].data[i] & vectors[1].data[i];
                }
                return Ok(res);
            }
        }

        let threshold = num_vectors / 2 + 1;
        let num_planes = (usize::BITS - num_vectors.leading_zeros()) as usize;

        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        // Performance Optimization: Use parallel bit-sliced addition for large batches (N >= 256).
        // For AVX2, we process 2 words (256 bits) per task to match register width.
        // For NEON, we process 1 word (128 bits) per task to match register width.
        if num_vectors >= 256 {
            #[cfg(target_arch = "x86_64")]
            if is_x86_feature_detected!("avx2") {
                let mut data = [0u128; 80];
                data.par_chunks_mut(2).enumerate().for_each(|(i, chunk)| {
                    // SAFETY: AVX2 is detected at runtime. Pointers to vectors and indices are within bounds.
                    let res = unsafe {
                        crate::hyperdim_simd_bundle::bundle_block_avx2_single(
                            vectors,
                            i * 2,
                            threshold,
                            num_planes,
                        )
                    };
                    // SAFETY: chunk length is 2 (32 bytes), matching AVX2 256-bit block size.
                    unsafe {
                        std::arch::x86_64::_mm256_storeu_si256(chunk.as_mut_ptr().cast(), res);
                    }
                });
                return Ok(Self { data });
            }

            #[cfg(target_arch = "aarch64")]
            {
                let mut data = [0u128; 80];
                data.par_iter_mut().enumerate().for_each(|(i, word)| {
                    // SAFETY: NEON is always available on aarch64. Pointers to vectors and indices are within bounds.
                    let res = unsafe {
                        crate::hyperdim_simd_bundle::bundle_block_neon_single(
                            vectors, i, threshold, num_planes,
                        )
                    };
                    // SAFETY: word is a single u128 (16 bytes), matching NEON 128-bit block size.
                    unsafe {
                        std::arch::aarch64::vst1q_u8(word as *mut u128 as *mut u8, res);
                    }
                });
                return Ok(Self { data });
            }

            // Parallel scalar fallback
            // Note: explicit gate for aarch64 to avoid unreachable code warnings
            // because the NEON path above returns unconditionally.
            #[cfg(not(target_arch = "aarch64"))]
            {
                let mut data = [0u128; 80];
                data.par_iter_mut().enumerate().for_each(|(i, word)| {
                    *word = bundle_word_scalar(vectors, i, threshold, num_planes);
                });
                return Ok(Self { data });
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 is detected at runtime.
            return Ok(Self {
                data: unsafe { bundle_block_avx2(vectors, threshold, num_planes) },
            });
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
        {
            // SAFETY: NEON is always available on aarch64.
            return Ok(Self {
                data: unsafe { bundle_block_neon(vectors, threshold, num_planes) },
            });
        }

        #[cfg(not(all(not(target_arch = "wasm32"), target_arch = "aarch64")))]
        {
            let mut data = [0u128; 80];
            #[allow(clippy::needless_range_loop)]
            for i in 0..80 {
                data[i] = bundle_word_scalar(vectors, i, threshold, num_planes);
            }
            Ok(Self { data })
        }
    }

    /// XOR binding of two hypervectors.
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
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let distance = self.hamming_distance(other);
        // Similarity = (Matches - Mismatches) / Dimension
        // Similarity = (Dimension - 2 * HammingDistance) / Dimension
        // Similarity = 1.0 - (2.0 * HammingDistance / 10240.0) = 1.0 - (HammingDistance / 5120.0)
        1.0 - (distance as f32 / 5120.0)
    }

    /// Hamming distance
    ///
    /// Dispatches to optimized SIMD paths based on platform:
    /// - x86_64: AVX2 (runtime detection) or unrolled scalar GPR popcount fallback
    /// - aarch64: NEON
    /// - Other: unrolled scalar GPR popcount
    #[must_use]
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 feature detected at runtime.
                unsafe { hamming_distance_simd_avx2(&self.data, &other.data) }
            } else {
                crate::hyperdim_simd::hamming_distance_optimized(&self.data, &other.data)
            }
        }

        #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
        {
            // SAFETY: aarch64 always has NEON.
            unsafe { hamming_distance_simd_neon(&self.data, &other.data) }
        }

        #[cfg(any(
            target_arch = "wasm32",
            not(any(target_arch = "x86_64", target_arch = "aarch64"))
        ))]
        {
            crate::hyperdim_simd::hamming_distance_optimized(&self.data, &other.data)
        }
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
        #[cfg(target_endian = "little")]
        {
            // Performance Optimization: [u128; 80] is bit-compatible with [u8; 1280]
            // on little-endian platforms. Using extend_from_slice with a casted
            // byte reference avoids 80 bounds checks and word-by-word serialization.
            // SAFETY: Alignment of u128 is stricter than u8. Pointers are valid.
            let data_bytes: &[u8; 1280] = unsafe { &*(self.data.as_ptr() as *const [u8; 1280]) };
            bytes.extend_from_slice(data_bytes);
        }
        #[cfg(not(target_endian = "little"))]
        {
            for word in &self.data {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
        bytes
    }

    /// Deserialize from bytes
    #[allow(clippy::missing_const_for_fn)] // Result return prevents const
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 1280 {
            return Err(crate::error::MemoryError::InvalidDimension {
                expected: 1280,
                actual: bytes.len(),
            });
        }

        #[allow(unused_mut)]
        let mut data = [0u128; 80];
        #[cfg(target_endian = "little")]
        {
            // Performance Optimization: Direct memcpy for little-endian platforms.
            // Avoids 80 loop iterations and multiple bounds checks per word.
            // SAFETY: bytes length is verified to be 1280. [u128; 80] is bit-compatible
            // with [u8; 1280] on little-endian. Pointers are valid.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.as_mut_ptr() as *mut u8, 1280);
            }
        }
        #[cfg(not(target_endian = "little"))]
        {
            for i in 0..80 {
                let mut word_bytes = [0u8; 16];
                word_bytes.copy_from_slice(&bytes[i * 16..(i + 1) * 16]);
                data[i] = u128::from_le_bytes(word_bytes);
            }
        }

        Ok(Self { data })
    }
}

// Serde impls are in hyperdim_serde.rs (LOC gate extraction)

// Re-export BundleAccumulator from bundle module
pub use crate::bundle::BundleAccumulator;
