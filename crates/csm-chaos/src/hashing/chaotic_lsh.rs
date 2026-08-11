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
    ///
    /// Performance Optimization: Unrolls the outer loop 8-way to process 8 projection rows
    /// simultaneously. This reduces L1 memory cache reads of `input` by 8x. The 8 dot products
    /// are consolidated into a single byte write per 8 iterations, minimizing memory writes
    /// and instruction overhead.
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    #[target_feature(enable = "avx2")]
    unsafe fn project_avx2(&self, input: &[f32]) -> [u64; 160] {
        use core::arch::x86_64::*;

        #[inline(always)]
        unsafe fn horizontal_sum_avx2(sum: __m256) -> f32 {
            unsafe {
                let hi = _mm256_extractf128_ps(sum, 1);
                let lo = _mm256_castps256_ps128(sum);
                let sum128 = _mm_add_ps(lo, hi);
                let shuf = _mm_movehdup_ps(sum128);
                let sums = _mm_add_ps(sum128, shuf);
                let shuf2 = _mm_movehl_ps(sums, sums);
                let result = _mm_add_ss(sums, shuf2);
                _mm_cvtss_f32(result)
            }
        }

        let mut bits = [0u64; 160];
        let len = self.input_dim;
        let chunks = len / 8;
        let remainder = len % 8;

        for i in (0..10240).step_by(8) {
            let offset0 = i * len;
            let offset1 = (i + 1) * len;
            let offset2 = (i + 2) * len;
            let offset3 = (i + 3) * len;
            let offset4 = (i + 4) * len;
            let offset5 = (i + 5) * len;
            let offset6 = (i + 6) * len;
            let offset7 = (i + 7) * len;

            let mut sum0 = _mm256_setzero_ps();
            let mut sum1 = _mm256_setzero_ps();
            let mut sum2 = _mm256_setzero_ps();
            let mut sum3 = _mm256_setzero_ps();
            let mut sum4 = _mm256_setzero_ps();
            let mut sum5 = _mm256_setzero_ps();
            let mut sum6 = _mm256_setzero_ps();
            let mut sum7 = _mm256_setzero_ps();

            for c in 0..chunks {
                let input_offset = c * 8;
                // SAFETY: Pointer arithmetic is bounded — input has `len` elements.
                let a = unsafe { _mm256_loadu_ps(input.as_ptr().add(input_offset)) };

                // SAFETY: projection_matrix has 10240 * len elements.
                // We load 8 independent segments corresponding to 8 different rows.
                unsafe {
                    let b0 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset0 + input_offset),
                    );
                    let b1 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset1 + input_offset),
                    );
                    let b2 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset2 + input_offset),
                    );
                    let b3 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset3 + input_offset),
                    );
                    let b4 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset4 + input_offset),
                    );
                    let b5 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset5 + input_offset),
                    );
                    let b6 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset6 + input_offset),
                    );
                    let b7 = _mm256_loadu_ps(
                        self.projection_matrix.as_ptr().add(offset7 + input_offset),
                    );

                    sum0 = _mm256_add_ps(sum0, _mm256_mul_ps(a, b0));
                    sum1 = _mm256_add_ps(sum1, _mm256_mul_ps(a, b1));
                    sum2 = _mm256_add_ps(sum2, _mm256_mul_ps(a, b2));
                    sum3 = _mm256_add_ps(sum3, _mm256_mul_ps(a, b3));
                    sum4 = _mm256_add_ps(sum4, _mm256_mul_ps(a, b4));
                    sum5 = _mm256_add_ps(sum5, _mm256_mul_ps(a, b5));
                    sum6 = _mm256_add_ps(sum6, _mm256_mul_ps(a, b6));
                    sum7 = _mm256_add_ps(sum7, _mm256_mul_ps(a, b7));
                }
            }

            // SAFETY: AVX2 horizontal sum is safe within the enabled target feature.
            let mut dot0 = unsafe { horizontal_sum_avx2(sum0) };
            let mut dot1 = unsafe { horizontal_sum_avx2(sum1) };
            let mut dot2 = unsafe { horizontal_sum_avx2(sum2) };
            let mut dot3 = unsafe { horizontal_sum_avx2(sum3) };
            let mut dot4 = unsafe { horizontal_sum_avx2(sum4) };
            let mut dot5 = unsafe { horizontal_sum_avx2(sum5) };
            let mut dot6 = unsafe { horizontal_sum_avx2(sum6) };
            let mut dot7 = unsafe { horizontal_sum_avx2(sum7) };

            // Scalar tail for remainder elements
            for r in 0..remainder {
                let idx = chunks * 8 + r;
                let val = input[idx];
                dot0 += val * self.projection_matrix[offset0 + idx];
                dot1 += val * self.projection_matrix[offset1 + idx];
                dot2 += val * self.projection_matrix[offset2 + idx];
                dot3 += val * self.projection_matrix[offset3 + idx];
                dot4 += val * self.projection_matrix[offset4 + idx];
                dot5 += val * self.projection_matrix[offset5 + idx];
                dot6 += val * self.projection_matrix[offset6 + idx];
                dot7 += val * self.projection_matrix[offset7 + idx];
            }

            let mut byte_val = 0u64;
            if dot0 > 0.0 {
                byte_val |= 1 << 0;
            }
            if dot1 > 0.0 {
                byte_val |= 1 << 1;
            }
            if dot2 > 0.0 {
                byte_val |= 1 << 2;
            }
            if dot3 > 0.0 {
                byte_val |= 1 << 3;
            }
            if dot4 > 0.0 {
                byte_val |= 1 << 4;
            }
            if dot5 > 0.0 {
                byte_val |= 1 << 5;
            }
            if dot6 > 0.0 {
                byte_val |= 1 << 6;
            }
            if dot7 > 0.0 {
                byte_val |= 1 << 7;
            }

            bits[i / 64] |= byte_val << (i % 64);
        }

        bits
    }

    /// NEON dot-product projection for aarch64.
    ///
    /// Performance Optimization: Unrolls the outer loop 8-way to process 8 projection rows
    /// simultaneously. This reduces L1 memory cache reads of `input` by 8x. The 8 dot products
    /// are consolidated into a single byte write per 8 iterations, minimizing memory writes
    /// and instruction overhead.
    #[cfg(target_arch = "aarch64")]
    #[inline]
    unsafe fn project_neon(&self, input: &[f32]) -> [u64; 160] {
        use core::arch::aarch64::*;

        let mut bits = [0u64; 160];
        let len = self.input_dim;
        let chunks = len / 4;
        let remainder = len % 4;

        for i in (0..10240).step_by(8) {
            let offset0 = i * len;
            let offset1 = (i + 1) * len;
            let offset2 = (i + 2) * len;
            let offset3 = (i + 3) * len;
            let offset4 = (i + 4) * len;
            let offset5 = (i + 5) * len;
            let offset6 = (i + 6) * len;
            let offset7 = (i + 7) * len;

            // SAFETY: target_feature "neon" is guaranteed by the cfg gate on this function.
            let mut sum0 = unsafe { vdupq_n_f32(0.0) };
            let mut sum1 = unsafe { vdupq_n_f32(0.0) };
            let mut sum2 = unsafe { vdupq_n_f32(0.0) };
            let mut sum3 = unsafe { vdupq_n_f32(0.0) };
            let mut sum4 = unsafe { vdupq_n_f32(0.0) };
            let mut sum5 = unsafe { vdupq_n_f32(0.0) };
            let mut sum6 = unsafe { vdupq_n_f32(0.0) };
            let mut sum7 = unsafe { vdupq_n_f32(0.0) };

            for c in 0..chunks {
                let input_offset = c * 4;
                // SAFETY: Pointer arithmetic bounded — input has `len` elements.
                let a = unsafe { vld1q_f32(input.as_ptr().add(input_offset)) };

                // SAFETY: projection_matrix has 10240 * len elements.
                unsafe {
                    let b0 = vld1q_f32(self.projection_matrix.as_ptr().add(offset0 + input_offset));
                    let b1 = vld1q_f32(self.projection_matrix.as_ptr().add(offset1 + input_offset));
                    let b2 = vld1q_f32(self.projection_matrix.as_ptr().add(offset2 + input_offset));
                    let b3 = vld1q_f32(self.projection_matrix.as_ptr().add(offset3 + input_offset));
                    let b4 = vld1q_f32(self.projection_matrix.as_ptr().add(offset4 + input_offset));
                    let b5 = vld1q_f32(self.projection_matrix.as_ptr().add(offset5 + input_offset));
                    let b6 = vld1q_f32(self.projection_matrix.as_ptr().add(offset6 + input_offset));
                    let b7 = vld1q_f32(self.projection_matrix.as_ptr().add(offset7 + input_offset));

                    sum0 = vfmaq_f32(sum0, a, b0);
                    sum1 = vfmaq_f32(sum1, a, b1);
                    sum2 = vfmaq_f32(sum2, a, b2);
                    sum3 = vfmaq_f32(sum3, a, b3);
                    sum4 = vfmaq_f32(sum4, a, b4);
                    sum5 = vfmaq_f32(sum5, a, b5);
                    sum6 = vfmaq_f32(sum6, a, b6);
                    sum7 = vfmaq_f32(sum7, a, b7);
                }
            }

            // SAFETY: target_feature "neon" is guaranteed by the cfg gate on this function.
            let mut dot0 = unsafe { vaddvq_f32(sum0) };
            let mut dot1 = unsafe { vaddvq_f32(sum1) };
            let mut dot2 = unsafe { vaddvq_f32(sum2) };
            let mut dot3 = unsafe { vaddvq_f32(sum3) };
            let mut dot4 = unsafe { vaddvq_f32(sum4) };
            let mut dot5 = unsafe { vaddvq_f32(sum5) };
            let mut dot6 = unsafe { vaddvq_f32(sum6) };
            let mut dot7 = unsafe { vaddvq_f32(sum7) };

            for r in 0..remainder {
                let idx = chunks * 4 + r;
                let val = input[idx];
                dot0 += val * self.projection_matrix[offset0 + idx];
                dot1 += val * self.projection_matrix[offset1 + idx];
                dot2 += val * self.projection_matrix[offset2 + idx];
                dot3 += val * self.projection_matrix[offset3 + idx];
                dot4 += val * self.projection_matrix[offset4 + idx];
                dot5 += val * self.projection_matrix[offset5 + idx];
                dot6 += val * self.projection_matrix[offset6 + idx];
                dot7 += val * self.projection_matrix[offset7 + idx];
            }

            let mut byte_val = 0u64;
            if dot0 > 0.0 {
                byte_val |= 1 << 0;
            }
            if dot1 > 0.0 {
                byte_val |= 1 << 1;
            }
            if dot2 > 0.0 {
                byte_val |= 1 << 2;
            }
            if dot3 > 0.0 {
                byte_val |= 1 << 3;
            }
            if dot4 > 0.0 {
                byte_val |= 1 << 4;
            }
            if dot5 > 0.0 {
                byte_val |= 1 << 5;
            }
            if dot6 > 0.0 {
                byte_val |= 1 << 6;
            }
            if dot7 > 0.0 {
                byte_val |= 1 << 7;
            }

            bits[i / 64] |= byte_val << (i % 64);
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
            #[allow(clippy::cast_precision_loss)]
            (1.0 - (dist as f32 / 5120.0))
        }

        let sim_12 = cosine_similarity(&h1, &h2);
        let sim_13 = cosine_similarity(&h1, &h3);

        assert!(
            sim_12 > 0.8,
            "Similar vectors should have high similarity: {sim_12}"
        );
        assert!(
            sim_13 < 0.6,
            "Different vectors should have low similarity: {sim_13}"
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
