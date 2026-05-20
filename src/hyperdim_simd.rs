//! SIMD-optimized hypervector operations.
//!
//! Provides platform-specific SIMD implementations for bind operations:
//! - x86/x86_64: SSE (128-bit) and AVX2 (256-bit) with runtime detection
//! - aarch64: NEON (128-bit)
//!
//! Also provides optimized Hamming distance calculation.

#[cfg(not(target_arch = "wasm32"))]
use crate::hyperdim::HVec10240;

/// AVX2-optimized bit-sliced bundling.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bundle_block_avx2(
    vectors: &[HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_andnot_si256, _mm256_loadu_si256, _mm256_or_si256,
        _mm256_set1_epi64x, _mm256_setzero_si256, _mm256_storeu_si256, _mm256_testz_si256,
        _mm256_xor_si256,
    };

    let mut out = [0u128; 80];
    for i in (0..80).step_by(2) {
        let mut planes = [_mm256_setzero_si256(); 64];
        for v in vectors {
            let mut carry = unsafe { _mm256_loadu_si256(v.data.as_ptr().add(i).cast()) };
            for plane in planes.iter_mut().take(num_planes) {
                let next_carry = _mm256_and_si256(*plane, carry);
                *plane = _mm256_xor_si256(*plane, carry);
                carry = next_carry;
                if _mm256_testz_si256(carry, carry) != 0 {
                    break;
                }
            }
        }

        let (mut current_eq, mut current_gt) = (_mm256_set1_epi64x(-1), _mm256_setzero_si256());
        for p in (0..num_planes).rev() {
            if ((threshold >> p) & 1) == 1 {
                current_eq = _mm256_and_si256(current_eq, planes[p]);
            } else {
                current_gt = _mm256_or_si256(current_gt, _mm256_and_si256(current_eq, planes[p]));
                current_eq = _mm256_andnot_si256(planes[p], current_eq);
            }
        }
        let res = _mm256_or_si256(current_gt, current_eq);
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// ARM NEON-optimized bit-sliced bundling.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn bundle_block_neon(
    vectors: &[HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::aarch64::{
        vandq_u8, vbicq_u8, vdupq_n_u8, veorq_u8, vgetq_lane_u64, vld1q_u8, vorrq_u8,
        vreinterpretq_u64_u8, vst1q_u8,
    };

    let mut out = [0u128; 80];
    for i in 0..80 {
        let mut planes = [vdupq_n_u8(0); 64];
        for v in vectors {
            let mut carry = unsafe { vld1q_u8(v.data.as_ptr().add(i).cast()) };
            for plane in planes.iter_mut().take(num_planes) {
                let next_carry = vandq_u8(*plane, carry);
                *plane = veorq_u8(*plane, carry);
                carry = next_carry;
                let c64 = vreinterpretq_u64_u8(carry);
                if vgetq_lane_u64(c64, 0) == 0 && vgetq_lane_u64(c64, 1) == 0 {
                    break;
                }
            }
        }

        let (mut current_eq, mut current_gt) = (vdupq_n_u8(0xFF), vdupq_n_u8(0));
        for p in (0..num_planes).rev() {
            if ((threshold >> p) & 1) == 1 {
                current_eq = vandq_u8(current_eq, planes[p]);
            } else {
                current_gt = vorrq_u8(current_gt, vandq_u8(current_eq, planes[p]));
                current_eq = vbicq_u8(current_eq, planes[p]);
            }
        }
        let res = vorrq_u8(current_gt, current_eq);
        unsafe { vst1q_u8(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

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

/// AVX2-optimized Hamming distance.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn hamming_distance_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    use std::arch::x86_64::{
        _mm256_add_epi8, _mm256_add_epi64, _mm256_and_si256, _mm256_loadu_si256, _mm256_sad_epu8,
        _mm256_set1_epi8, _mm256_setr_epi8, _mm256_setzero_si256, _mm256_shuffle_epi8,
        _mm256_srli_epi16, _mm256_storeu_si256, _mm256_xor_si256,
    };

    let mut total_count = _mm256_setzero_si256();
    let low_mask = _mm256_set1_epi8(0x0F);
    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );

    for i in (0..80).step_by(2) {
        let a = unsafe { _mm256_loadu_si256(lhs.as_ptr().add(i).cast()) };
        let b = unsafe { _mm256_loadu_si256(rhs.as_ptr().add(i).cast()) };
        let x = _mm256_xor_si256(a, b);

        let low = _mm256_and_si256(x, low_mask);
        let high = _mm256_and_si256(_mm256_srli_epi16(x, 4), low_mask);

        let pop_low = _mm256_shuffle_epi8(lookup, low);
        let pop_high = _mm256_shuffle_epi8(lookup, high);

        let combined = _mm256_add_epi8(pop_low, pop_high);
        total_count = _mm256_add_epi64(
            total_count,
            _mm256_sad_epu8(combined, _mm256_setzero_si256()),
        );
    }

    let mut out = [0u64; 4];
    unsafe { _mm256_storeu_si256(out.as_mut_ptr().cast(), total_count) };
    (out[0] + out[1] + out[2] + out[3]) as u32
}

/// ARM NEON-optimized Hamming distance.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn hamming_distance_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    use std::arch::aarch64::{
        vaddq_u32, vaddvq_u32, vcntq_u8, vdupq_n_u32, veorq_u8, vld1q_u8, vpaddlq_u8, vpaddlq_u16,
    };
    let mut total = vdupq_n_u32(0);
    for i in 0..80 {
        let (a, b) = unsafe {
            (
                vld1q_u8(lhs.as_ptr().add(i).cast()),
                vld1q_u8(rhs.as_ptr().add(i).cast()),
            )
        };
        let x = veorq_u8(a, b);
        let pop = vcntq_u8(x);
        let sum = vpaddlq_u8(pop);
        let sum2 = vpaddlq_u16(sum);
        total = vaddq_u32(total, sum2);
    }
    vaddvq_u32(total)
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

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn hamming_distance_simd_avx2_correctness() {
        if std::arch::is_x86_feature_detected!("avx2") {
            let (lhs, rhs) = make_test_vectors();
            let scalar = hamming_distance_optimized(&lhs, &rhs);
            let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
            assert_eq!(simd, scalar);

            use crate::hyperdim::HVec10240;
            for i in 0..10 {
                let v1 = HVec10240::new_seeded(i);
                let v2 = HVec10240::new_seeded(i + 100);
                let scalar_r = hamming_distance_optimized(&v1.data, &v2.data);
                let simd_r = unsafe { hamming_distance_simd_avx2(&v1.data, &v2.data) };
                assert_eq!(simd_r, scalar_r);
            }
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn bundle_block_avx2_correctness() {
        if std::arch::is_x86_feature_detected!("avx2") {
            use crate::hyperdim::HVec10240;
            let vectors: Vec<HVec10240> = (0..10).map(HVec10240::new_seeded).collect();
            let threshold = vectors.len() / 2 + 1;
            let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
            let simd_res = unsafe { bundle_block_avx2(&vectors, threshold, num_planes) };
            let mut expected = [0u128; 80];
            for i in 0..80 {
                let mut planes = [0u128; 64];
                for v in &vectors {
                    let mut carry = v.data[i];
                    for p in 0..num_planes {
                        let next_carry = planes[p] & carry;
                        planes[p] ^= carry;
                        carry = next_carry;
                    }
                }
                let (mut current_eq, mut current_gt) = (!0u128, 0u128);
                for p in (0..num_planes).rev() {
                    if ((threshold >> p) & 1) == 1 {
                        current_eq &= planes[p];
                    } else {
                        current_gt |= current_eq & planes[p];
                        current_eq &= !planes[p];
                    }
                }
                expected[i] = current_gt | current_eq;
            }
            assert_eq!(simd_res, expected);
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
