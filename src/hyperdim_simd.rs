//! SIMD-optimized hypervector operations.
//!
//! Provides platform-specific SIMD implementations for bind operations:
//! - x86/x86_64: SSE (128-bit) and AVX2 (256-bit) with runtime detection
//! - aarch64: NEON (128-bit)
//!
//! Also provides optimized Hamming distance calculation.

/// Optimized Hamming distance calculation using unrolled loop.
///
/// This implementation uses a 4x unrolled loop with independent accumulators
/// to break the serial dependency chain of popcount operations, maximizing
/// Instruction-Level Parallelism (ILP). It operates on 64-bit words to avoid
/// the overhead of 128-bit operations on many architectures.
#[inline]
pub(crate) fn hamming_distance_optimized(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let distance: u32;
    unsafe {
        let lptr = lhs.as_ptr() as *const u64;
        let rptr = rhs.as_ptr() as *const u64;
        let mut s0 = 0;
        let mut s1 = 0;
        let mut s2 = 0;
        let mut s3 = 0;
        for i in (0..160).step_by(4) {
            s0 += (*lptr.add(i) ^ *rptr.add(i)).count_ones();
            s1 += (*lptr.add(i + 1) ^ *rptr.add(i + 1)).count_ones();
            s2 += (*lptr.add(i + 2) ^ *rptr.add(i + 2)).count_ones();
            s3 += (*lptr.add(i + 3) ^ *rptr.add(i + 3)).count_ones();
        }
        distance = (s0 + s1) + (s2 + s3);
    }
    distance
}

/// SSE-optimized bind (128-bit XOR).
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
pub(crate) fn bind_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128};
    let mut out = [0u128; 80];
    for i in 0..80 {
        unsafe {
            let a = _mm_loadu_si128((&lhs[i] as *const u128).cast::<__m128i>());
            let b = _mm_loadu_si128((&rhs[i] as *const u128).cast::<__m128i>());
            let x = _mm_xor_si128(a, b);
            _mm_storeu_si128((&mut out[i] as *mut u128).cast::<__m128i>(), x);
        }
    }
    out
}

/// SSE-optimized bitwise AND (128-bit).
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
pub(crate) fn and_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m128i, _mm_and_si128, _mm_loadu_si128, _mm_storeu_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m128i, _mm_and_si128, _mm_loadu_si128, _mm_storeu_si128};
    let mut out = [0u128; 80];
    for i in 0..80 {
        unsafe {
            let a = _mm_loadu_si128((&lhs[i] as *const u128).cast::<__m128i>());
            let b = _mm_loadu_si128((&rhs[i] as *const u128).cast::<__m128i>());
            let x = _mm_and_si128(a, b);
            _mm_storeu_si128((&mut out[i] as *mut u128).cast::<__m128i>(), x);
        }
    }
    out
}

/// AVX2-optimized batch Hamming distance.
///
/// Computes Hamming distances between a single query and a batch of candidates.
/// Uses AVX2 to XOR 256 bits (2 x u128) at a time.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn batch_hamming_distance_avx2(
    query: &[u128; 80],
    candidates: &[crate::hyperdim::HVec10240],
    out: &mut [u32],
) {
    use std::arch::x86_64::{_mm256_loadu_si256, _mm256_xor_si256};

    for (candidate, distance) in candidates.iter().zip(out.iter_mut()) {
        let qptr = query.as_ptr() as *const std::arch::x86_64::__m256i;
        let cptr = candidate.data.as_ptr() as *const std::arch::x86_64::__m256i;
        let mut s0 = 0u64;
        let mut s1 = 0u64;

        // Process 80 u128 words = 40 u256 blocks
        for i in (0..40).step_by(2) {
            let (x0, x1) = unsafe {
                let q0 = _mm256_loadu_si256(qptr.add(i));
                let c0 = _mm256_loadu_si256(cptr.add(i));
                let x0 = _mm256_xor_si256(q0, c0);

                let q1 = _mm256_loadu_si256(qptr.add(i + 1));
                let c1 = _mm256_loadu_si256(cptr.add(i + 1));
                let x1 = _mm256_xor_si256(q1, c1);
                (x0, x1)
            };

            // Extract to u64 for popcount
            let v0: [u64; 4] = unsafe { std::mem::transmute(x0) };
            let v1: [u64; 4] = unsafe { std::mem::transmute(x1) };

            s0 += v0[0].count_ones() as u64;
            s1 += v0[1].count_ones() as u64;
            s0 += v0[2].count_ones() as u64;
            s1 += v0[3].count_ones() as u64;

            s0 += v1[0].count_ones() as u64;
            s1 += v1[1].count_ones() as u64;
            s0 += v1[2].count_ones() as u64;
            s1 += v1[3].count_ones() as u64;
        }
        *distance = (s0 + s1) as u32;
    }
}

/// AVX2-optimized bitwise AND (256-bit).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn and_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::x86_64::{__m256i, _mm256_and_si256, _mm256_loadu_si256, _mm256_storeu_si256};
    let mut out = [0u128; 80];
    for i in (0..80).step_by(2) {
        unsafe {
            let ptr_lhs = lhs.as_ptr().add(i) as *const __m256i;
            let ptr_rhs = rhs.as_ptr().add(i) as *const __m256i;
            let ptr_out = out.as_mut_ptr().add(i) as *mut __m256i;
            let a = _mm256_loadu_si256(ptr_lhs);
            let b = _mm256_loadu_si256(ptr_rhs);
            let x = _mm256_and_si256(a, b);
            _mm256_storeu_si256(ptr_out, x);
        }
    }
    out
}

/// ARM NEON-optimized bitwise AND (128-bit).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn and_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{vandq_u64, vld1q_u64, vst1q_u64};
    let mut out = [0u128; 80];
    for i in 0..80 {
        unsafe {
            let lhs_ptr = lhs.as_ptr().add(i) as *const u64;
            let rhs_ptr = rhs.as_ptr().add(i) as *const u64;
            let out_ptr = out.as_mut_ptr().add(i) as *mut u64;
            let a = vld1q_u64(lhs_ptr);
            let b = vld1q_u64(rhs_ptr);
            let x = vandq_u64(a, b);
            vst1q_u64(out_ptr, x);
        }
    }
    out
}

/// AVX2-optimized bind (256-bit XOR, processes 2 words per instruction).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::x86_64::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256, _mm256_xor_si256};
    let mut out = [0u128; 80];
    for i in (0..80).step_by(2) {
        unsafe {
            let ptr_lhs = lhs.as_ptr().add(i) as *const __m256i;
            let ptr_rhs = rhs.as_ptr().add(i) as *const __m256i;
            let ptr_out = out.as_mut_ptr().add(i) as *mut __m256i;
            let a = _mm256_loadu_si256(ptr_lhs);
            let b = _mm256_loadu_si256(ptr_rhs);
            let x = _mm256_xor_si256(a, b);
            _mm256_storeu_si256(ptr_out, x);
        }
    }
    out
}

/// ARM NEON-optimized bind (128-bit XOR).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{veorq_u64, vld1q_u64, vst1q_u64};
    let mut out = [0u128; 80];
    for i in 0..80 {
        unsafe {
            let lhs_ptr = lhs.as_ptr().add(i) as *const u64;
            let rhs_ptr = rhs.as_ptr().add(i) as *const u64;
            let out_ptr = out.as_mut_ptr().add(i) as *mut u64;
            let a = vld1q_u64(lhs_ptr);
            let b = vld1q_u64(rhs_ptr);
            let x = veorq_u64(a, b);
            vst1q_u64(out_ptr, x);
        }
    }
    out
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn test_batch_hamming_distance_avx2_consistency() {
        if std::arch::is_x86_feature_detected!("avx2") {
            let query = HVec10240::random();
            let candidates: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
            let mut distances = vec![0u32; candidates.len()];

            unsafe {
                batch_hamming_distance_avx2(&query.data, &candidates, &mut distances);
            }

            for (i, candidate) in candidates.iter().enumerate() {
                let expected = query.hamming_distance(candidate);
                assert_eq!(distances[i], expected, "Distance mismatch at index {}", i);
            }
        }
    }

    fn make_test_vectors() -> ([u128; 80], [u128; 80]) {
        let mut lhs = [0u128; 80];
        let mut rhs = [0u128; 80];
        for i in 0..80 {
            lhs[i] = (i as u128) * 0x123456789ABCDEF;
            rhs[i] = (i as u128) * 0xFEDCBA987654321;
        }
        (lhs, rhs)
    }

    #[test]
    fn hamming_distance_optimized_correctness() {
        let lhs = [0xFFFFFFFFFFFFFFFF_FFFFFFFFFFFFFFFFu128; 80];
        let rhs = [0u128; 80];
        let distance = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(distance, 10240);
    }

    #[test]
    fn hamming_distance_optimized_identical_vectors() {
        let v = [0x123456789ABCDEF_0FEDCBA987654321u128; 80];
        let distance = hamming_distance_optimized(&v, &v);
        assert_eq!(distance, 0);
    }

    #[test]
    fn hamming_distance_optimized_complements() {
        let lhs = [0xAAAAAAAAAAAAAAAA_AAAAAAAAAAAAAAAAu128; 80];
        let rhs = [0x5555555555555555_5555555555555555u128; 80];
        let distance = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(distance, 10240);
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_arch = "x86_64", target_arch = "x86")
    ))]
    #[test]
    fn bind_simd_x86_correctness() {
        let (lhs, rhs) = make_test_vectors();
        let result = bind_simd_x86(&lhs, &rhs);
        for i in 0..80 {
            assert_eq!(result[i], lhs[i] ^ rhs[i]);
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn bind_simd_avx2_correctness() {
        let (lhs, rhs) = make_test_vectors();
        if std::arch::is_x86_feature_detected!("avx2") {
            let result = unsafe { bind_simd_avx2(&lhs, &rhs) };
            for i in 0..80 {
                assert_eq!(result[i], lhs[i] ^ rhs[i]);
            }
            let sse_result = bind_simd_x86(&lhs, &rhs);
            assert_eq!(result, sse_result);
        }
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_arch = "x86_64", target_arch = "x86")
    ))]
    #[test]
    fn and_simd_x86_correctness() {
        let (lhs, rhs) = make_test_vectors();
        let result = and_simd_x86(&lhs, &rhs);
        for i in 0..80 {
            assert_eq!(result[i], lhs[i] & rhs[i]);
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn and_simd_avx2_correctness() {
        let (lhs, rhs) = make_test_vectors();
        if std::arch::is_x86_feature_detected!("avx2") {
            let result = unsafe { and_simd_avx2(&lhs, &rhs) };
            for i in 0..80 {
                assert_eq!(result[i], lhs[i] & rhs[i]);
            }
            let sse_result = and_simd_x86(&lhs, &rhs);
            assert_eq!(result, sse_result);
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    #[test]
    fn and_simd_neon_correctness() {
        let (lhs, rhs) = make_test_vectors();
        let result = unsafe { and_simd_neon(&lhs, &rhs) };
        for i in 0..80 {
            assert_eq!(result[i], lhs[i] & rhs[i]);
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    #[test]
    fn bind_simd_neon_correctness() {
        let (lhs, rhs) = make_test_vectors();
        let result = unsafe { bind_simd_neon(&lhs, &rhs) };
        for i in 0..80 {
            assert_eq!(result[i], lhs[i] ^ rhs[i]);
        }
    }

    #[test]
    fn hamming_distance_matches_bit_count() {
        let lhs: [u128; 80] = std::array::from_fn(|i| 1u128 << (i % 128));
        let rhs: [u128; 80] = std::array::from_fn(|i| 1u128 << ((i + 64) % 128));
        let distance = hamming_distance_optimized(&lhs, &rhs);
        let expected: u32 = lhs
            .iter()
            .zip(rhs.iter())
            .map(|(l, r)| (l ^ r).count_ones())
            .sum();
        assert_eq!(distance, expected);
    }
}
