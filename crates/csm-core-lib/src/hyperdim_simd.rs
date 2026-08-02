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
    const LOADS_PER_FLUSH: usize = 20;
    const TOTAL_LOADS: usize = 40;
    const UNROLL_FACTOR: usize = 2;
    // Compile-time guard for loop structure and overflow safety.
    // Max bits per byte = 8. 8 * UNROLL_FACTOR * (LOADS_PER_FLUSH / UNROLL_FACTOR) = 8 * 20 = 160.
    // 160 safely fits in u8 (255) to prevent overflow during deferred accumulation.
    const _: () = assert!(80 % (LOADS_PER_FLUSH * 2) == 0);
    const _: () = assert!(LOADS_PER_FLUSH % (UNROLL_FACTOR * 2) == 0);

    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );
    let low_mask = _mm256_set1_epi8(0x0f);
    let mut acc = _mm256_setzero_si256();
    let zero = _mm256_setzero_si256();

    // Algorithmic Optimization: Deferred 8-bit accumulation with dual accumulators and unrolling.
    // 80 words = 40 AVX2 loads. We process in two 20-load flushes to avoid 8-bit overflow.
    // Dual accumulators (acc_8_low, acc_8_high) and 2x unrolling improve ILP by exposing
    // independent execution paths to the scheduler.
    for i in (0..80).step_by(LOADS_PER_FLUSH * 2) {
        let mut acc_8_low = _mm256_setzero_si256();
        let mut acc_8_high = _mm256_setzero_si256();
        for j in (0..LOADS_PER_FLUSH * 2).step_by(UNROLL_FACTOR * 2) {
            let idx0 = i + j;
            let idx1 = idx0 + 2;

            // SAFETY: i + j + 2 is at most 40 + 36 + 2 = 78.
            // _mm256_loadu_si256 reads 32 bytes (2 x u128), so add(78) reads indices [78, 79].
            // This stays within the bounds of the 80-element input arrays.
            unsafe {
                let x0 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx0).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx0).cast()),
                );
                let x1 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx1).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx1).cast()),
                );

                acc_8_low = _mm256_add_epi8(
                    acc_8_low,
                    _mm256_add_epi8(
                        _mm256_shuffle_epi8(lookup, _mm256_and_si256(x0, low_mask)),
                        _mm256_shuffle_epi8(lookup, _mm256_and_si256(x1, low_mask)),
                    ),
                );
                acc_8_high = _mm256_add_epi8(
                    acc_8_high,
                    _mm256_add_epi8(
                        _mm256_shuffle_epi8(
                            lookup,
                            _mm256_and_si256(_mm256_srli_epi16(x0, 4), low_mask),
                        ),
                        _mm256_shuffle_epi8(
                            lookup,
                            _mm256_and_si256(_mm256_srli_epi16(x1, 4), low_mask),
                        ),
                    ),
                );
            }
        }
        acc = _mm256_add_epi64(
            acc,
            _mm256_add_epi64(
                _mm256_sad_epu8(acc_8_low, zero),
                _mm256_sad_epu8(acc_8_high, zero),
            ),
        );
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
        vaddlvq_u16, vaddq_u8, vaddq_u16, vcntq_u8, vdupq_n_u8, vdupq_n_u16, veorq_u8, vld1q_u8,
        vpaddlq_u8,
    };
    const BATCH_SIZE: usize = 10;
    const WORDS_PER_BATCH: usize = BATCH_SIZE * 2;
    // Compile-time guard for array alignment
    const _: () = assert!(80 % WORDS_PER_BATCH == 0);

    let mut acc = vdupq_n_u16(0);

    for i in (0..80).step_by(WORDS_PER_BATCH) {
        // Algorithmic Optimization: Intermediate 8-bit accumulation for NEON.
        // We accumulate popcounts in an 8-bit vector for 10 iterations (20 words)
        // before flushing to the 16-bit accumulator via vpaddlq_u8.
        // Max bits per byte lane is 8. Over 20 additions (10 iterations * 2 loads),
        // max sum is 8 * 20 = 160, which safely fits in u8 (255).
        // This reduces the frequency of vpaddlq_u8 (widening pairwise add) calls by 10x.
        let mut acc_8 = vdupq_n_u8(0);
        for j in 0..BATCH_SIZE {
            let idx = i + j * 2;
            // SAFETY: idx and idx+1 are in (0..80). vld1q_u8 loads 16 bytes.
            unsafe {
                let l0 = vld1q_u8(lhs.as_ptr().add(idx).cast());
                let r0 = vld1q_u8(rhs.as_ptr().add(idx).cast());
                let x0 = veorq_u8(l0, r0);
                let c0 = vcntq_u8(x0);
                acc_8 = vaddq_u8(acc_8, c0);

                let l1 = vld1q_u8(lhs.as_ptr().add(idx + 1).cast());
                let r1 = vld1q_u8(rhs.as_ptr().add(idx + 1).cast());
                let x1 = veorq_u8(l1, r1);
                let c1 = vcntq_u8(x1);
                acc_8 = vaddq_u8(acc_8, c1);
            }
        }
        acc = vaddq_u16(acc, vpaddlq_u8(acc_8));
    }
    vaddlvq_u16(acc) as u32
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn hamming_distance_binary_simd_avx2(lhs: &[u64; 160], rhs: &[u64; 160]) -> u32 {
    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );
    let low_mask = _mm256_set1_epi8(0x0f);
    let mut acc = _mm256_setzero_si256();
    let zero = _mm256_setzero_si256();

    for i in (0..160).step_by(80) {
        let mut acc_8_low = _mm256_setzero_si256();
        let mut acc_8_high = _mm256_setzero_si256();
        for j in (0..80).step_by(16) {
            let idx0 = i + j;
            let idx1 = idx0 + 4;
            let idx2 = idx0 + 8;
            let idx3 = idx0 + 12;

            unsafe {
                let x0 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx0).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx0).cast()),
                );
                let x1 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx1).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx1).cast()),
                );
                let x2 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx2).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx2).cast()),
                );
                let x3 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.as_ptr().add(idx3).cast()),
                    _mm256_loadu_si256(rhs.as_ptr().add(idx3).cast()),
                );

                let low01 = _mm256_add_epi8(
                    _mm256_shuffle_epi8(lookup, _mm256_and_si256(x0, low_mask)),
                    _mm256_shuffle_epi8(lookup, _mm256_and_si256(x1, low_mask)),
                );
                let low23 = _mm256_add_epi8(
                    _mm256_shuffle_epi8(lookup, _mm256_and_si256(x2, low_mask)),
                    _mm256_shuffle_epi8(lookup, _mm256_and_si256(x3, low_mask)),
                );
                acc_8_low = _mm256_add_epi8(acc_8_low, _mm256_add_epi8(low01, low23));

                let high01 = _mm256_add_epi8(
                    _mm256_shuffle_epi8(
                        lookup,
                        _mm256_and_si256(_mm256_srli_epi16(x0, 4), low_mask),
                    ),
                    _mm256_shuffle_epi8(
                        lookup,
                        _mm256_and_si256(_mm256_srli_epi16(x1, 4), low_mask),
                    ),
                );
                let high23 = _mm256_add_epi8(
                    _mm256_shuffle_epi8(
                        lookup,
                        _mm256_and_si256(_mm256_srli_epi16(x2, 4), low_mask),
                    ),
                    _mm256_shuffle_epi8(
                        lookup,
                        _mm256_and_si256(_mm256_srli_epi16(x3, 4), low_mask),
                    ),
                );
                acc_8_high = _mm256_add_epi8(acc_8_high, _mm256_add_epi8(high01, high23));
            }
        }
        acc = _mm256_add_epi64(
            acc,
            _mm256_add_epi64(
                _mm256_sad_epu8(acc_8_low, zero),
                _mm256_sad_epu8(acc_8_high, zero),
            ),
        );
    }

    let mut results = [0u64; 4];
    unsafe {
        _mm256_storeu_si256(results.as_mut_ptr().cast(), acc);
    }
    (results[0] + results[1] + results[2] + results[3]) as u32
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn hamming_distance_binary_simd_neon(lhs: &[u64; 160], rhs: &[u64; 160]) -> u32 {
    use std::arch::aarch64::{
        vaddlvq_u16, vaddq_u8, vaddq_u16, vcntq_u8, vdupq_n_u8, vdupq_n_u16, veorq_u8, vld1q_u8,
        vpaddlq_u8,
    };
    const BATCH_SIZE: usize = 10;
    const WORDS_PER_BATCH: usize = BATCH_SIZE * 4;
    let mut acc = vdupq_n_u16(0);

    for i in (0..160).step_by(WORDS_PER_BATCH) {
        let mut acc_8 = vdupq_n_u8(0);
        for j in 0..BATCH_SIZE {
            let idx = i + j * 4;
            let l0 = vld1q_u8(lhs.as_ptr().add(idx).cast());
            let r0 = vld1q_u8(rhs.as_ptr().add(idx).cast());
            let x0 = veorq_u8(l0, r0);
            let c0 = vcntq_u8(x0);
            acc_8 = vaddq_u8(acc_8, c0);

            let l1 = vld1q_u8(lhs.as_ptr().add(idx + 2).cast());
            let r1 = vld1q_u8(rhs.as_ptr().add(idx + 2).cast());
            let x1 = veorq_u8(l1, r1);
            let c1 = vcntq_u8(x1);
            acc_8 = vaddq_u8(acc_8, c1);
        }
        acc = vaddq_u16(acc, vpaddlq_u8(acc_8));
    }
    vaddlvq_u16(acc) as u32
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn simd_avx2_correctness() {
        if is_x86_feature_detected!("avx2") {
            let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
            let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
            assert!(unsafe { and_simd_avx2(&lhs, &rhs) }.iter().all(|&w| w == 0));
            assert!(
                unsafe { bind_simd_avx2(&lhs, &lhs) }
                    .iter()
                    .all(|&w| w == 0)
            );
            assert_eq!(
                unsafe { hamming_distance_simd_avx2(&[0u128; 80], &[!0u128; 80]) },
                10240
            );
            let mut r = [0u128; 80];
            r[0] = 1;
            r[79] = 1 << 127;
            assert_eq!(unsafe { hamming_distance_simd_avx2(&[0u128; 80], &r) }, 2);
        }
    }

    #[test]
    #[cfg(all(
        not(target_arch = "wasm32"),
        any(target_arch = "x86_64", target_arch = "x86")
    ))]
    fn simd_x86_correctness() {
        let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
        assert!(and_simd_x86(&lhs, &rhs).iter().all(|&w| w == 0));
        assert!(bind_simd_x86(&lhs, &lhs).iter().all(|&w| w == 0));
    }

    #[test]
    fn hamming_distance_optimized_correctness() {
        let mut rhs = [0u128; 80];
        for i in 0..80 {
            rhs[i] = i as u128;
        }
        let expected = rhs.iter().map(|&x| x.count_ones()).sum::<u32>();
        assert_eq!(hamming_distance_optimized(&[0u128; 80], &rhs), expected);

        let vec = [0x123456789ABCDEF0u128; 80];
        assert_eq!(hamming_distance_optimized(&vec, &vec), 0);

        let vec5 = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let veca = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
        assert_eq!(hamming_distance_optimized(&vec5, &veca), 10240);

        let mut lhs = [0u128; 80];
        let mut r = [0u128; 80];
        lhs[0] = 0b1010;
        r[0] = 0b1100;
        assert_eq!(hamming_distance_optimized(&lhs, &r), 2);
    }
}
