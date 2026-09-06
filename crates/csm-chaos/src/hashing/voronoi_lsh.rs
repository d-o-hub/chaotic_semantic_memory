//! Voronoi Diagram Encoded Hashing (VDeH) using chaotic centroids.
//!
//! Implements the hashing scheme based on "VDeH: Voronoi Diagram Encoded Hashing
//! for Effective and Efficient Similarity Search" (DOI: 10.1613/jair.1.21934).
//! It generates data-independent pseudo-random Voronoi partitions using 2D-SLHM
//! to map continuous inputs into a 10240-bit binary hypervector space.
//!
//! SIMD-accelerated on x86_64 (AVX2) and aarch64 (NEON) with scalar fallback.

use crate::maps::hyperchaotic::Slhm2d;
use alloc::vec::Vec;

/// Voronoi Diagram Encoded LSH projector using chaotic centroids.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VoronoiLsh {
    /// Stores the paired centroids for each bit.
    /// For 10240 bits, there are 10240 * 2 = 20480 centroids.
    /// A flattened array where each centroid takes `input_dim` contiguous floats.
    centroids: Vec<f32>,
    input_dim: usize,
}

impl VoronoiLsh {
    /// Create a new `VoronoiLsh` with chaotic generation for its Voronoi partitions.
    /// Each bit requires two centroids; the bit is set if the input is closer to centroid A than centroid B.
    pub fn new(x: f64, y: f64, a: f64, input_dim: usize) -> Self {
        let mut map = Slhm2d::new(x, y, a);
        // 10240 bits * 2 centroids per bit * input_dim
        let total_floats = 10240 * 2 * input_dim;
        let mut centroids = Vec::with_capacity(total_floats);

        for _ in 0..total_floats {
            #[allow(clippy::cast_possible_truncation)]
            centroids.push((map.next_value() * 2.0 - 1.0) as f32);
        }

        Self {
            centroids,
            input_dim,
        }
    }

    /// Scalar Voronoi projection (reference implementation).
    pub fn project_scalar(&self, input: &[f32]) -> [u64; 160] {
        let mut bits = [0u64; 160];
        if input.is_empty() || input.len() != self.input_dim {
            return bits;
        }

        let dim = self.input_dim;
        let stride = 2 * dim; // 2 centroids per bit

        for i in 0..10240 {
            let offset_a = i * stride;
            let offset_b = offset_a + dim;

            let mut dist_a = 0.0f32;
            let mut dist_b = 0.0f32;

            for j in 0..dim {
                let diff_a = input[j] - self.centroids[offset_a + j];
                dist_a += diff_a * diff_a;

                let diff_b = input[j] - self.centroids[offset_b + j];
                dist_b += diff_b * diff_b;
            }

            // Encode: 1 if closer to centroid A, 0 if closer to centroid B.
            if dist_a < dist_b {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        bits
    }

    /// SIMD-accelerated Voronoi projection. Uses AVX2/NEON when available, scalar fallback otherwise.
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

    /// Project an input vector into a binary hypervector using Voronoi diagram encoded hashing.
    /// Dispatches to SIMD when available at runtime.
    pub fn project(&self, input: &[f32]) -> [u64; 160] {
        self.project_simd(input)
    }

    /// AVX2 L2-distance projection for x86_64.
    ///
    /// Performance Optimization: Unrolls the inner dimension loop 2-way with dual YMM accumulators
    /// (`sum_a0`, `sum_a1` and `sum_b0`, `sum_b1`). This breaks latency dependency chains on YMM
    /// additions and exposes instruction-level parallelism (ILP) to the CPU execution ports.
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
        let dim = self.input_dim;
        let stride = 2 * dim;
        let chunks2 = dim / 16;
        let remainder_chunks = (dim % 16) / 8;
        let remainder = dim % 8;

        for i in 0..10240 {
            let offset_a = i * stride;
            let offset_b = offset_a + dim;

            let mut sum_a0 = _mm256_setzero_ps();
            let mut sum_a1 = _mm256_setzero_ps();
            let mut sum_b0 = _mm256_setzero_ps();
            let mut sum_b1 = _mm256_setzero_ps();

            for c in 0..chunks2 {
                let idx0 = c * 16;
                let idx1 = idx0 + 8;
                // SAFETY: Pointer arithmetic is bounded.
                unsafe {
                    let v_in0 = _mm256_loadu_ps(input.as_ptr().add(idx0));
                    let v_in1 = _mm256_loadu_ps(input.as_ptr().add(idx1));

                    let v_a0 = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_a + idx0));
                    let v_a1 = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_a + idx1));
                    let v_b0 = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_b + idx0));
                    let v_b1 = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_b + idx1));

                    let diff_a0 = _mm256_sub_ps(v_in0, v_a0);
                    let diff_a1 = _mm256_sub_ps(v_in1, v_a1);
                    sum_a0 = _mm256_add_ps(sum_a0, _mm256_mul_ps(diff_a0, diff_a0));
                    sum_a1 = _mm256_add_ps(sum_a1, _mm256_mul_ps(diff_a1, diff_a1));

                    let diff_b0 = _mm256_sub_ps(v_in0, v_b0);
                    let diff_b1 = _mm256_sub_ps(v_in1, v_b1);
                    sum_b0 = _mm256_add_ps(sum_b0, _mm256_mul_ps(diff_b0, diff_b0));
                    sum_b1 = _mm256_add_ps(sum_b1, _mm256_mul_ps(diff_b1, diff_b1));
                }
            }

            if remainder_chunks > 0 {
                let idx = chunks2 * 16;
                unsafe {
                    let v_in = _mm256_loadu_ps(input.as_ptr().add(idx));
                    let v_a = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_a + idx));
                    let v_b = _mm256_loadu_ps(self.centroids.as_ptr().add(offset_b + idx));

                    let diff_a = _mm256_sub_ps(v_in, v_a);
                    sum_a0 = _mm256_add_ps(sum_a0, _mm256_mul_ps(diff_a, diff_a));
                    let diff_b = _mm256_sub_ps(v_in, v_b);
                    sum_b0 = _mm256_add_ps(sum_b0, _mm256_mul_ps(diff_b, diff_b));
                }
            }

            // SAFETY: AVX2 horizontal sums.
            let sum_a = _mm256_add_ps(sum_a0, sum_a1);
            let sum_b = _mm256_add_ps(sum_b0, sum_b1);
            let mut dist_a = unsafe { horizontal_sum_avx2(sum_a) };
            let mut dist_b = unsafe { horizontal_sum_avx2(sum_b) };

            // Scalar tail for remainder elements
            let tail_start = chunks2 * 16 + remainder_chunks * 8;
            for r in 0..remainder {
                let idx = tail_start + r;
                let diff_a_tail = input[idx] - self.centroids[offset_a + idx];
                dist_a += diff_a_tail * diff_a_tail;
                let diff_b_tail = input[idx] - self.centroids[offset_b + idx];
                dist_b += diff_b_tail * diff_b_tail;
            }

            if dist_a < dist_b {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }

        bits
    }

    /// NEON L2-distance projection for aarch64.
    ///
    /// Performance Optimization: Unrolls the inner dimension loop 2-way with dual NEON accumulators
    /// (`sum_a0`, `sum_a1` and `sum_b0`, `sum_b1`). This breaks latency dependency chains on NEON
    /// additions and exposes instruction-level parallelism (ILP) to the CPU execution ports.
    #[cfg(target_arch = "aarch64")]
    #[inline]
    unsafe fn project_neon(&self, input: &[f32]) -> [u64; 160] {
        use core::arch::aarch64::*;

        let mut bits = [0u64; 160];
        let dim = self.input_dim;
        let stride = 2 * dim;
        let chunks2 = dim / 8;
        let remainder_chunks = (dim % 8) / 4;
        let remainder = dim % 4;

        for i in 0..10240 {
            let offset_a = i * stride;
            let offset_b = offset_a + dim;

            // SAFETY: target_feature "neon" is guaranteed by the cfg gate.
            let mut sum_a0 = unsafe { vdupq_n_f32(0.0) };
            let mut sum_a1 = unsafe { vdupq_n_f32(0.0) };
            let mut sum_b0 = unsafe { vdupq_n_f32(0.0) };
            let mut sum_b1 = unsafe { vdupq_n_f32(0.0) };

            for c in 0..chunks2 {
                let idx0 = c * 8;
                let idx1 = idx0 + 4;
                // SAFETY: Pointer arithmetic bounded.
                unsafe {
                    let v_in0 = vld1q_f32(input.as_ptr().add(idx0));
                    let v_in1 = vld1q_f32(input.as_ptr().add(idx1));

                    let v_a0 = vld1q_f32(self.centroids.as_ptr().add(offset_a + idx0));
                    let v_a1 = vld1q_f32(self.centroids.as_ptr().add(offset_a + idx1));
                    let v_b0 = vld1q_f32(self.centroids.as_ptr().add(offset_b + idx0));
                    let v_b1 = vld1q_f32(self.centroids.as_ptr().add(offset_b + idx1));

                    let diff_a0 = vsubq_f32(v_in0, v_a0);
                    let diff_a1 = vsubq_f32(v_in1, v_a1);
                    sum_a0 = vfmaq_f32(sum_a0, diff_a0, diff_a0);
                    sum_a1 = vfmaq_f32(sum_a1, diff_a1, diff_a1);

                    let diff_b0 = vsubq_f32(v_in0, v_b0);
                    let diff_b1 = vsubq_f32(v_in1, v_b1);
                    sum_b0 = vfmaq_f32(sum_b0, diff_b0, diff_b0);
                    sum_b1 = vfmaq_f32(sum_b1, diff_b1, diff_b1);
                }
            }

            if remainder_chunks > 0 {
                let idx = chunks2 * 8;
                unsafe {
                    let v_in = vld1q_f32(input.as_ptr().add(idx));
                    let v_a = vld1q_f32(self.centroids.as_ptr().add(offset_a + idx));
                    let v_b = vld1q_f32(self.centroids.as_ptr().add(offset_b + idx));

                    let diff_a = vsubq_f32(v_in, v_a);
                    sum_a0 = vfmaq_f32(sum_a0, diff_a, diff_a);

                    let diff_b = vsubq_f32(v_in, v_b);
                    sum_b0 = vfmaq_f32(sum_b0, diff_b, diff_b);
                }
            }

            // SAFETY: target_feature "neon" is guaranteed by the cfg gate.
            let sum_a = unsafe { vaddq_f32(sum_a0, sum_a1) };
            let sum_b = unsafe { vaddq_f32(sum_b0, sum_b1) };
            let mut dist_a = unsafe { vaddvq_f32(sum_a) };
            let mut dist_b = unsafe { vaddvq_f32(sum_b) };

            let tail_start = chunks2 * 8 + remainder_chunks * 4;
            for r in 0..remainder {
                let idx = tail_start + r;
                let diff_a_tail = input[idx] - self.centroids[offset_a + idx];
                dist_a += diff_a_tail * diff_a_tail;
                let diff_b_tail = input[idx] - self.centroids[offset_b + idx];
                dist_b += diff_b_tail * diff_b_tail;
            }

            if dist_a < dist_b {
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
    fn test_voronoi_lsh_locality() {
        let input_dim = 4;
        let lsh = VoronoiLsh::new(0.123, 0.456, 0.99, input_dim);

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
        let lsh = VoronoiLsh::new(0.5, 0.7, 0.95, 8);
        let input = [0.1, -0.3, 0.5, 0.7, -0.2, 0.4, -0.6, 0.8];
        let scalar = lsh.project_scalar(&input);
        let result = lsh.project(&input);
        assert_eq!(scalar, result);
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_voronoi_lsh_determinism() {
        let lsh1 = VoronoiLsh::new(0.5, 0.6, 3.99, 128);
        let lsh2 = VoronoiLsh::new(0.5, 0.6, 3.99, 128);

        let input = vec![0.1f32; 128];
        assert_eq!(lsh1.project(&input), lsh2.project(&input));
    }

    #[test]
    fn test_voronoi_lsh_finite_behavior() {
        let lsh = VoronoiLsh::new(0.1, 0.2, 3.9, 16);
        let input = vec![f32::INFINITY; 16];
        let bits = lsh.project(&input);

        // Ensure it doesn't panic on infinity and returns a valid hypervector,
        // even if it's all zeros or some defined state (NaNs drop out safely).
        assert_eq!(bits.len(), 160);
    }
}
