//! SIMD-optimized hypervector operations.
//!
//! Provides AVX2, x86-SSE, and ARM NEON paths for common HDC primitives.

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
use std::arch::x86_64::{
    _mm256_and_si256, _mm256_loadu_si256, _mm256_storeu_si256, _mm256_xor_si256,
};

#[allow(dead_code)]
pub(crate) fn hamming_distance_optimized(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let mut distance = 0;
    for i in 0..80 {
        distance += (lhs[i] ^ rhs[i]).count_ones();
    }
    distance
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn and_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in (0..80).step_by(2) {
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
pub(crate) unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    let mut res = [0u128; 80];
    for i in (0..80).step_by(2) {
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
pub(crate) unsafe fn hamming_distance_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let mut dist = 0u32;
    for i in 0..80 {
        dist += (lhs[i] ^ rhs[i]).count_ones();
    }
    dist
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
pub(crate) unsafe fn and_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{vandq_u8, vld1q_u8, vst1q_u8};
    let mut res = [0u128; 80];
    for i in 0..80 {
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
pub(crate) unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{veorq_u8, vld1q_u8, vst1q_u8};
    let mut res = [0u128; 80];
    for i in 0..80 {
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
pub(crate) unsafe fn hamming_distance_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {
    let mut dist = 0u32;
    for i in 0..80 {
        dist += (lhs[i] ^ rhs[i]).count_ones();
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn and_simd_avx2_correctness() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
            let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
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
