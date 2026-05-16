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

/// AVX2-optimized bit-sliced bundling block.
/// Processes two 128-bit words across all vectors.
///
/// Algorithmic Optimization: Process 256 bits at a time using AVX2.
/// Uses bit-sliced addition to count set bits across vectors.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bundle_block_avx2(
    vectors: &[crate::hyperdim::HVec10240],
    threshold: usize,
    num_planes: usize,
    word_idx: usize,
) -> [u128; 2] {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_andnot_si256, _mm256_loadu_si256, _mm256_or_si256,
        _mm256_set1_epi32, _mm256_storeu_si256, _mm256_xor_si256,
    };

    // Use 64 planes to match scalar implementation and support up to 2^64 vectors.
    let mut planes = [_mm256_set1_epi32(0); 64];

    for v in vectors {
        let mut carry = unsafe { _mm256_loadu_si256(v.data.as_ptr().add(word_idx).cast()) };
        for plane in planes.iter_mut().take(num_planes) {
            let next_carry = _mm256_and_si256(*plane, carry);
            *plane = _mm256_xor_si256(*plane, carry);
            carry = next_carry;
        }
    }

    let mut current_eq = _mm256_set1_epi32(-1); // All ones
    let mut current_gt = _mm256_set1_epi32(0);

    for p in (0..num_planes).rev() {
        if ((threshold >> p) & 1) == 1 {
            current_eq = _mm256_and_si256(current_eq, planes[p]);
        } else {
            current_gt = _mm256_or_si256(current_gt, _mm256_and_si256(current_eq, planes[p]));
            current_eq = _mm256_andnot_si256(planes[p], current_eq);
        }
    }

    let res = _mm256_or_si256(current_gt, current_eq);
    let mut out = [0u128; 2];
    unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), res) };
    out
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
