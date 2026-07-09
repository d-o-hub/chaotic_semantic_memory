//! Chaotic Locality-Sensitive Hashing (LSH).
//!
//! Projects continuous vectors into binary hypervectors using 2D-SLHM trajectories.
//! SIMD-accelerated on x86_64 (AVX2) and aarch64 (NEON) with scalar fallback.

use crate::maps::hyperchaotic::Slhm2d;
use alloc::vec::Vec;

/// Chaotic LSH projector using 2D-SLHM.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChaoticLsh {
    projection_matrix: Vec<f32>,
    input_dim: usize,
}

impl ChaoticLsh {
    /// Create a new ChaoticLsh with given seed values and input dimension.
    pub fn new(x: f64, y: f64, a: f64, input_dim: usize) -> Self {
        let mut map = Slhm2d::new(x, y, a);
        let mut projection_matrix = Vec::with_capacity(10240 * input_dim);

        for _ in 0..10240 {
            for _ in 0..input_dim {
                #[allow(clippy::cast_possible_truncation)]
                // Intentional f64→f32 for projection values
                projection_matrix.push((map.next_value() * 2.0 - 1.0) as f32);
            }
        }

        Self {
            projection_matrix,
            input_dim,
        }
    }

    /// Scalar dot-product projection (reference implementation).
    pub fn project_scalar(&self, input: &[f32]) -> [u64; 160] {
        let mut bits = [0u64; 160];
        if input.is_empty() || input.len() != self.input_dim {
            return bits;
        }
        for i in 0..10240 {
            let mut dot_product = 0.0f32;
            let offset = i * self.input_dim;
            for (j, &val) in input.iter().enumerate() {
                dot_product += val * self.projection_matrix[offset + j];
            }
            if dot_product > 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        bits
    }

    /// SIMD-accelerated projection. Uses AVX2/NEON when available, scalar fallback otherwise.
    pub fn project_simd(&self, input: &[f32]) -> [u64; 160] {
        if input.is_empty() || input.len() != self.input_dim {
            return [0u64; 160];
        }

        #[cfg(all(target_arch = "x86_64", feature = "std"))]
        {
            if std::arch::is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 feature detected at runtime before calling.
                return unsafe { self.project_avx2(input) };
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // SAFETY: NEON is always available on aarch64.
            return unsafe { self.project_neon(input) };
        }

        #[allow(unreachable_code)]
        self.project_scalar(input)
    }

    /// Project an input vector into a binary hypervector.
    /// Dispatches to SIMD when available at runtime.
    pub fn project(&self, input: &[f32]) -> [u64; 160] {
        self.project_simd(input)
    }

    /// AVX2 dot-product projection for x86_64.
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    #[target_feature(enable = "avx2")]
    unsafe fn project_avx2(&self, input: &[f32]) -> [u64; 160] {
        use core::arch::x86_64::*;

        let mut bits = [0u64; 160];
        let len = self.input_dim;
        let chunks = len / 8;
        let remainder = len % 8;

        for i in 0..10240 {
            let offset = i * len;
            let mut sum = _mm256_setzero_ps();

            for c in 0..chunks {
                let base = offset + c * 8;
                // SAFETY: Pointer arithmetic is bounded — input has `len` elements and
                // projection_matrix has 10240 * len elements. c*8 < len and
                // offset + base < 10240*len for all iterations.
                unsafe {
                    let a = _mm256_loadu_ps(input.as_ptr().add(c * 8));
                    let b = _mm256_loadu_ps(self.projection_matrix.as_ptr().add(base));
                    sum = _mm256_add_ps(sum, _mm256_mul_ps(a, b));
                }
            }

            // Horizontal sum of 8 f32 lanes
            let hi = _mm256_extractf128_ps(sum, 1);
            let lo = _mm256_castps256_ps128(sum);
            let sum128 = _mm_add_ps(lo, hi);
            let shuf = _mm_movehdup_ps(sum128);
            let sums = _mm_add_ps(sum128, shuf);
            let shuf2 = _mm_movehl_ps(sums, sums);
            let result = _mm_add_ss(sums, shuf2);
            let mut dot_product = _mm_cvtss_f32(result);

            // Scalar tail for remainder elements
            for r in 0..remainder {
                let idx = chunks * 8 + r;
                dot_product += input[idx] * self.projection_matrix[offset + idx];
            }

            if dot_product > 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }

        bits
    }

    /// NEON dot-product projection for aarch64.
    #[cfg(target_arch = "aarch64")]
    #[inline]
    unsafe fn project_neon(&self, input: &[f32]) -> [u64; 160] {
        use core::arch::aarch64::*;

        let mut bits = [0u64; 160];
        let len = self.input_dim;
        let chunks = len / 4;
        let remainder = len % 4;

        for i in 0..10240 {
            let offset = i * len;
            // SAFETY: target_feature "neon" is guaranteed by the cfg gate on this function.
            let mut sum = unsafe { vdupq_n_f32(0.0) };

            for c in 0..chunks {
                let base = offset + c * 4;
                // SAFETY: Pointer arithmetic bounded — input has `len` elements,
                // projection_matrix has 10240 * len elements.
                unsafe {
                    let a = vld1q_f32(input.as_ptr().add(c * 4));
                    let b = vld1q_f32(self.projection_matrix.as_ptr().add(base));
                    sum = vfmaq_f32(sum, a, b);
                }
            }

            // SAFETY: target_feature "neon" is guaranteed by the cfg gate on this function.
            let mut dot_product = unsafe { vaddvq_f32(sum) };

            for r in 0..remainder {
                let idx = chunks * 4 + r;
                dot_product += input[idx] * self.projection_matrix[offset + idx];
            }

            if dot_product > 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }

        bits
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_chaotic_lsh_locality() {
        let input_dim = 4;
        let lsh = ChaoticLsh::new(0.123, 0.456, 0.99, input_dim);

        let v1 = [1.0, 0.5, -0.2, 0.8];
        let v2 = [1.0, 0.5, -0.2, 0.81];
        let v3 = [-1.0, -0.5, 0.2, -0.8];

        let h1 = lsh.project(&v1);
        let h2 = lsh.project(&v2);
        let h3 = lsh.project(&v3);

        fn hamming(a: &[u64; 160], b: &[u64; 160]) -> u32 {
            let mut dist = 0u32;
            for i in 0..160 {
                dist += (a[i] ^ b[i]).count_ones();
            }
            dist
        }

        fn cosine_similarity(a: &[u64; 160], b: &[u64; 160]) -> f32 {
            let dist = hamming(a, b);
            1.0 - (dist as f32 / 5120.0)
        }

        let sim_12 = cosine_similarity(&h1, &h2);
        let sim_13 = cosine_similarity(&h1, &h3);

        assert!(
            sim_12 > 0.8,
            "Similar vectors should have high similarity: {}",
            sim_12
        );
        assert!(
            sim_13 < 0.6,
            "Different vectors should have low similarity: {}",
            sim_13
        );
    }

    #[test]
    fn test_simd_scalar_parity() {
        let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 8);
        let input = [0.1, -0.3, 0.5, 0.7, -0.2, 0.4, -0.6, 0.8];
        let scalar = lsh.project_scalar(&input);
        let result = lsh.project(&input);
        assert_eq!(scalar, result);
    }
}
