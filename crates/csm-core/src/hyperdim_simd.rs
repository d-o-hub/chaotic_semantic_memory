//! SIMD-optimized hypervector operations.
//!
//! Provides AVX2, x86-SSE, and ARM NEON paths for common HDC primitives.

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
use std::arch::x86_64::{
    _mm256_add_epi8, _mm256_add_epi64, _mm256_and_si256, _mm256_loadu_si256, _mm256_sad_epu8,
    _mm256_set1_epi8, _mm256_setr_epi8, _mm256_setzero_si256, _mm256_shuffle_epi8,
    _mm256_srli_epi16, _mm256_storeu_si256, _mm256_xor_si256,
};

#[allow(dead_code)]
pub(crate) fn hamming_distance_optimized(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let mut d0 = 0;
    let mut d1 = 0;
    let mut d2 = 0;
    let mut d3 = 0;

    // Unroll by 4 with independent accumulators to improve ILP
    for i in (0..80).step_by(4) {
        d0 += (lhs[i] ^ rhs[i]).count_ones();
        d1 += (lhs[i + 1] ^ rhs[i + 1]).count_ones();
        d2 += (lhs[i + 2] ^ rhs[i + 2]).count_ones();
        d3 += (lhs[i + 3] ^ rhs[i + 3]).count_ones();
    }
    d0 + d1 + d2 + d3
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn and_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in (0..80).step_by(2) {
        // SAFETY: lhs, rhs, and res are [u128; 80], which is 1280 bytes.
        // i goes up to 78, so i+2 (256 bits) is 32 bytes.
        // 32 bytes * 40 iterations = 1280 bytes. All pointers are valid.
        unsafe {
            let l = _mm256_loadu_si256(lhs.as_ptr().add(i).cast());
            let r = _mm256_loadu_si256(rhs.as_ptr().add(i).cast());
            _mm256_storeu_si256(res.as_mut_ptr().add(i).cast(), _mm256_and_si256(l, r));
        }
    }
    res
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in (0..80).step_by(2) {
        // SAFETY: lhs, rhs, and res are [u128; 80], which is 1280 bytes.
        // i goes up to 78, so i+2 (256 bits) is 32 bytes.
        // 32 bytes * 40 iterations = 1280 bytes. All pointers are valid.
        unsafe {
            let l = _mm256_loadu_si256(lhs.as_ptr().add(i).cast());
            let r = _mm256_loadu_si256(rhs.as_ptr().add(i).cast());
            _mm256_storeu_si256(res.as_mut_ptr().add(i).cast(), _mm256_xor_si256(l, r));
        }
    }
    res
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn hamming_distance_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );
    let low_mask = _mm256_set1_epi8(0x0f);
    let mut acc = _mm256_setzero_si256();
    let zero = _mm256_setzero_si256();

    // Deferred accumulation: accumulate 10 x 256-bit chunks in 8-bit registers
    // (max sum 10*8=80 < 255) before flushing to 64-bit accumulators via SAD.
    // This reduces the frequency of expensive _mm256_sad_epu8 and _mm256_add_epi64.
    for i in (0..80).step_by(20) {
        let mut local_acc = _mm256_setzero_si256();
        for j in 0..10 {
            // SAFETY: lhs and rhs are [u128; 80]. add(i + j * 2) is safe for i in 0..4 and j in 0..10.
            // _mm256_loadu_si256 loads 256 bits (32 bytes).
            unsafe {
                let l = _mm256_loadu_si256(lhs.as_ptr().add(i + j * 2).cast());
                let r = _mm256_loadu_si256(rhs.as_ptr().add(i + j * 2).cast());
                let x = _mm256_xor_si256(l, r);

                let low = _mm256_and_si256(x, low_mask);
                let high = _mm256_and_si256(_mm256_srli_epi16(x, 4), low_mask);

                let pop_low = _mm256_shuffle_epi8(lookup, low);
                let pop_high = _mm256_shuffle_epi8(lookup, high);

                local_acc = _mm256_add_epi8(local_acc, _mm256_add_epi8(pop_low, pop_high));
            }
        }
        acc = _mm256_add_epi64(acc, _mm256_sad_epu8(local_acc, zero));
    }

    let mut results = [0u64; 4];
    unsafe {
        _mm256_storeu_si256(results.as_mut_ptr().cast(), acc);
    }
    #[allow(clippy::cast_possible_truncation)]
    let res = (results[0] + results[1] + results[2] + results[3]) as u32;
    res
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
pub(crate) fn and_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in 0..80 {
        res[i] = lhs[i] & rhs[i];
    }
    res
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_arch = "x86_64", target_arch = "x86")
))]
#[inline]
pub(crate) fn bind_simd_x86(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in 0..80 {
        res[i] = lhs[i] ^ rhs[i];
    }
    res
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn and_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{vandq_u8, vld1q_u8, vst1q_u8};
    let mut res = [0u128; 80];
    for i in 0..80 {
        // SAFETY: lhs, rhs, and res are [u128; 80]. vld1q_u8 loads 128 bits (16 bytes).
        // add(i) moves the pointer by i * sizeof(u128), which is exactly 16 bytes.
        // All accesses are within bounds.
        unsafe {
            let l = vld1q_u8(lhs.as_ptr().add(i).cast());
            let r = vld1q_u8(rhs.as_ptr().add(i).cast());
            vst1q_u8(res.as_mut_ptr().add(i).cast(), vandq_u8(l, r));
        }
    }
    res
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{veorq_u8, vld1q_u8, vst1q_u8};
    let mut res = [0u128; 80];
    for i in 0..80 {
        // SAFETY: lhs, rhs, and res are [u128; 80]. vld1q_u8 loads 128 bits (16 bytes).
        // add(i) moves the pointer by i * sizeof(u128), which is exactly 16 bytes.
        // All accesses are within bounds.
        unsafe {
            let l = vld1q_u8(lhs.as_ptr().add(i).cast());
            let r = vld1q_u8(rhs.as_ptr().add(i).cast());
            vst1q_u8(res.as_mut_ptr().add(i).cast(), veorq_u8(l, r));
        }
    }
    res
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn hamming_distance_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    use std::arch::aarch64::{
        vaddlvq_u16, vaddq_u16, vcntq_u8, vdupq_n_u16, veorq_u8, vld1q_u8, vpaddlq_u8,
    };
    let mut acc = vdupq_n_u16(0);

    for i in (0..80).step_by(2) {
        // SAFETY: lhs and rhs are [u128; 80]. vld1q_u8 loads 128 bits (16 bytes).
        // add(i) moves the pointer by i * sizeof(u128), which is exactly 16 bytes.
        // All accesses are within bounds.
        unsafe {
            let l0 = vld1q_u8(lhs.as_ptr().add(i).cast());
            let r0 = vld1q_u8(rhs.as_ptr().add(i).cast());
            let x0 = veorq_u8(l0, r0);
            let c0 = vcntq_u8(x0);
            acc = vaddq_u16(acc, vpaddlq_u8(c0));

            let l1 = vld1q_u8(lhs.as_ptr().add(i + 1).cast());
            let r1 = vld1q_u8(rhs.as_ptr().add(i + 1).cast());
            let x1 = veorq_u8(l1, r1);
            let c1 = vcntq_u8(x1);
            acc = vaddq_u16(acc, vpaddlq_u8(c1));
        }
    }
    vaddlvq_u16(acc) as u32
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn and_simd_avx2_correctness() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
            let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
            // SAFETY: AVX2 support is checked above.
            let res = unsafe { and_simd_avx2(&lhs, &rhs) };
            for word in &res {
                assert_eq!(*word, 0);
            }
        }
    }

    #[test]
    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_arch = "x86_64", target_arch = "x86")
    ))]
    fn and_simd_x86_correctness() {
        let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
        let res = and_simd_x86(&lhs, &rhs);
        for word in &res {
            assert_eq!(*word, 0);
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn bind_simd_avx2_correctness() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
            let rhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
            // SAFETY: AVX2 support is checked above.
            let res = unsafe { bind_simd_avx2(&lhs, &rhs) };
            for word in &res {
                assert_eq!(*word, 0);
            }
        }
    }

    #[test]
    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_arch = "x86_64", target_arch = "x86")
    ))]
    fn bind_simd_x86_correctness() {
        let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let rhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let res = bind_simd_x86(&lhs, &rhs);
        for word in &res {
            assert_eq!(*word, 0);
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn hamming_distance_simd_avx2_correctness() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0u128; 80];
            let rhs = [!0u128; 80];
            // SAFETY: AVX2 support is checked above.
            let res = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
            assert_eq!(res, 10240);
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn hamming_distance_simd_avx2_edge_cases() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0u128; 80];
            let mut rhs = [0u128; 80];
            rhs[0] = 1;
            rhs[79] = 1 << 127;
            // SAFETY: AVX2 support is checked above.
            let res = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
            assert_eq!(res, 2);
        }
    }

    #[test]
    fn hamming_distance_matches_bit_count() {
        let lhs = [0u128; 80];
        let mut rhs = [0u128; 80];
        for i in 0..80 {
            rhs[i] = i as u128;
        }
        let res = hamming_distance_optimized(&lhs, &rhs);
        let mut expected = 0;
        for i in 0..80 {
            expected += (rhs[i]).count_ones();
        }
        assert_eq!(res, expected);
    }

    #[test]
    fn hamming_distance_optimized_identical_vectors() {
        let vec = [0x123456789ABCDEF0u128; 80];
        assert_eq!(hamming_distance_optimized(&vec, &vec), 0);
    }

    #[test]
    fn hamming_distance_optimized_complements() {
        let vec = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let complement = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
        assert_eq!(hamming_distance_optimized(&vec, &complement), 10240);
    }

    #[test]
    fn hamming_distance_optimized_correctness() {
        let mut lhs = [0u128; 80];
        let mut rhs = [0u128; 80];
        lhs[0] = 0b1010;
        rhs[0] = 0b1100;
        // 1010 ^ 1100 = 0110 (2 bits set)
        assert_eq!(hamming_distance_optimized(&lhs, &rhs), 2);
    }
}
