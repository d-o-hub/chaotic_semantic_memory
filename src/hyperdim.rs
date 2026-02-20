//! Hyperdimensional computing primitives
//!
//! Implements 10240-bit hypervectors using `[u128; 80]`.

use rand::Rng;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::error::Result;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
fn bind_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};

    let mut out = [0u128; 80];
    for i in 0..80 {
        // SAFETY: `u128` is 16-byte aligned, matching `__m128i` requirements.
        // Pointers come from fixed-size arrays with at least 16 bytes per element.
        unsafe {
            let a = _mm_loadu_si128((&lhs[i] as *const u128).cast::<__m128i>());
            let b = _mm_loadu_si128((&rhs[i] as *const u128).cast::<__m128i>());
            let x = _mm_xor_si128(a, b);
            _mm_storeu_si128((&mut out[i] as *mut u128).cast::<__m128i>(), x);
        }
    }
    out
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
fn cosine_similarity_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> f32 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};

    let mut dot_product: u32 = 0;
    for i in 0..80 {
        let mut lanes = [0u64; 2];
        // SAFETY: `u128` is 16-byte aligned, matching `__m128i` requirements.
        // `lanes` provides a 16-byte writable region (2 x u64 = 16 bytes).
        unsafe {
            let a = _mm_loadu_si128((&lhs[i] as *const u128).cast::<__m128i>());
            let b = _mm_loadu_si128((&rhs[i] as *const u128).cast::<__m128i>());
            let x = _mm_xor_si128(a, b);
            _mm_storeu_si128(lanes.as_mut_ptr().cast::<__m128i>(), x);
        }
        dot_product += (!lanes[0]).count_ones() + (!lanes[1]).count_ones();
    }
    (2.0 * dot_product as f32 / HVec10240::DIMENSION as f32) - 1.0
}

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
    pub fn zero() -> Self {
        Self { data: [0u128; 80] }
    }

    /// Create a random hypervector (each bit has 50% probability)
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut data = [0u128; 80];
        for word in &mut data {
            *word = rng.r#gen();
        }
        Self { data }
    }

    /// Create a random sparse hypervector with given density
    pub fn sparse(density: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut data = [0u128; 80];
        let bits_to_set = (Self::DIMENSION as f32 * density) as usize;

        for _ in 0..bits_to_set {
            let pos = rng.gen_range(0..Self::DIMENSION);
            let word = pos / 128;
            let bit = pos % 128;
            data[word] |= 1u128 << bit;
        }

        Self { data }
    }

    /// Bundle (sum) multiple hypervectors
    pub fn bundle(vectors: &[Self]) -> Result<Self> {
        if vectors.is_empty() {
            return Ok(Self::zero());
        }

        #[cfg(not(target_arch = "wasm32"))]
        let counts = vectors
            .par_iter()
            .fold(
                || Box::new([0i32; Self::DIMENSION]),
                |mut local, v| {
                    #[allow(clippy::needless_range_loop)]
                    for i in 0..80 {
                        for j in 0..128 {
                            if (v.data[i] >> j) & 1 == 1 {
                                local[i * 128 + j] += 1;
                            }
                        }
                    }
                    local
                },
            )
            .reduce(
                || Box::new([0i32; Self::DIMENSION]),
                |mut a, b| {
                    #[allow(clippy::needless_range_loop)]
                    for i in 0..Self::DIMENSION {
                        a[i] += b[i];
                    }
                    a
                },
            );

        #[cfg(target_arch = "wasm32")]
        let counts = {
            let mut local = Box::new([0i32; Self::DIMENSION]);
            for v in vectors {
                #[allow(clippy::needless_range_loop)]
                for i in 0..80 {
                    for j in 0..128 {
                        if (v.data[i] >> j) & 1 == 1 {
                            local[i * 128 + j] += 1;
                        }
                    }
                }
            }
            local
        };

        let threshold = vectors.len() as i32 / 2;
        let mut data = [0u128; 80];
        #[allow(clippy::needless_range_loop)]
        for i in 0..Self::DIMENSION {
            if counts[i] > threshold {
                let word = i / 128;
                let bit = i % 128;
                data[word] |= 1u128 << bit;
            }
        }

        Ok(Self { data })
    }

    /// XOR binding of two hypervectors
    pub fn bind(&self, other: &Self) -> Self {
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(target_arch = "x86_64", target_arch = "x86")
        ))]
        {
            Self {
                data: bind_simd_x86(&self.data, &other.data),
            }
        }

        #[cfg(not(all(
            not(target_arch = "wasm32"),
            any(target_arch = "x86_64", target_arch = "x86")
        )))]
        {
            let mut result = [0u128; 80];
            for i in 0..80 {
                result[i] = self.data[i] ^ other.data[i];
            }
            Self { data: result }
        }
    }

    /// Cosine similarity between two hypervectors
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        #[cfg(all(
            not(target_arch = "wasm32"),
            any(target_arch = "x86_64", target_arch = "x86")
        ))]
        {
            cosine_similarity_simd_x86(&self.data, &other.data)
        }

        #[cfg(not(all(
            not(target_arch = "wasm32"),
            any(target_arch = "x86_64", target_arch = "x86")
        )))]
        {
            let mut dot_product: u32 = 0;
            for i in 0..80 {
                let eq = !(self.data[i] ^ other.data[i]);
                dot_product += eq.count_ones();
            }
            (2.0 * dot_product as f32 / Self::DIMENSION as f32) - 1.0
        }
    }

    /// Hamming distance
    #[must_use]
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        let mut distance = 0u32;
        for i in 0..80 {
            distance += (self.data[i] ^ other.data[i]).count_ones();
        }
        distance
    }

    /// Permute the hypervector (rotation)
    pub fn permute(&self, shift: usize) -> Self {
        let mut result = [0u128; 80];
        let bit_shift = shift % 128;
        let word_shift = (shift / 128) % 80;

        for (i, word) in result.iter_mut().enumerate() {
            let src1 = (i + word_shift) % 80;
            if bit_shift == 0 {
                *word = self.data[src1];
            } else {
                let src2 = (i + word_shift + 1) % 80;
                *word = (self.data[src1] << bit_shift) | (self.data[src2] >> (128 - bit_shift));
            }
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
        let bytes = self.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

struct HVecVisitor;

impl<'de> Visitor<'de> for HVecVisitor {
    type Value = HVec10240;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a byte array of length 1280")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        HVec10240::from_bytes(v).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(HVecVisitor)
    }
}

/// Batch similarity computation with optimized chunked parallelism.
/// Uses Rayon par_chunks() with tuned chunk size for cache efficiency.
/// Benchmark target: <500μs for 1000 candidates.
pub fn batch_cosine_similarity(query: &HVec10240, candidates: &[HVec10240]) -> Vec<f32> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        // Tuned chunk size: 32 candidates fits well in L1 cache
        // Each candidate = 1280 bytes, 32 candidates = ~40KB (fits L1)
        const CHUNK_SIZE: usize = 32;
        let mut results = vec![0.0f32; candidates.len()];
        candidates
            .par_chunks(CHUNK_SIZE)
            .zip(results.par_chunks_mut(CHUNK_SIZE))
            .for_each(|(cands, out)| {
                // Sequential processing within chunk for cache efficiency
                for (i, c) in cands.iter().enumerate() {
                    out[i] = query.cosine_similarity(c);
                }
            });
        results
    }
    #[cfg(target_arch = "wasm32")]
    {
        candidates
            .iter()
            .map(|c| query.cosine_similarity(c))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hvec_creation() {
        let vec = HVec10240::zero();
        assert_eq!(vec.data.iter().sum::<u128>(), 0);
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
        assert!(similarity > 0.99);
    }

    #[test]
    fn test_binding() {
        let a = HVec10240::random();
        let b = HVec10240::random();
        let bound = a.bind(&b);
        let recovered = bound.bind(&b);
        let similarity = a.cosine_similarity(&recovered);
        assert!(similarity > 0.95);
    }

    #[test]
    fn test_serialization() {
        let v = HVec10240::random();
        let bytes = v.to_bytes();
        assert_eq!(v.data, HVec10240::from_bytes(&bytes).unwrap().data);
    }

    #[test]
    fn test_bundle() {
        let v: Vec<_> = (0..10).map(|_| HVec10240::random()).collect();
        assert_eq!(HVec10240::bundle(&v).unwrap().data.len(), 80);
    }

    #[test]
    fn test_permute() {
        let v = HVec10240::random();
        assert_eq!(v, v.permute(0));
        let s = v.permute(128);
        for i in 0..80 {
            assert_eq!(s.data[i], v.data[(i + 1) % 80]);
        }
    }
}
