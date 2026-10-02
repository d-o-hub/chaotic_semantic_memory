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


    for i in (0..80).step_by(4) {
        d0 += (lhs[i] ^ rhs[i]).count_ones();
        d1 += (lhs[i + 1] ^ rhs[i + 1]).count_ones();
        d2 += (lhs[i + 2] ^ rhs[i + 2]).count_ones();
        d3 += (lhs[i + 3] ^ rhs[i + 3]).count_ones();
    }
    d0 + d1 + d2 + d3
}

/// Hamming distance over packed `[u64; 160]` words (the `BHVec10240` layout).
///
/// Dispatches to the AVX2 (x86_64) or NEON (aarch64) kernels over the raw
/// 1,280 bytes of the packed words when available, falling back to an unrolled
/// scalar popcount loop on other targets (including wasm32). No intermediate
/// layout conversion is performed.
#[inline]
pub(crate) fn hamming_distance_u64(lhs: &[u64; 160], rhs: &[u64; 160]) -> u32 {
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    if std::is_x86_feature_detected!("avx2") {

        return unsafe { hamming_distance_1280_avx2(lhs.as_ptr().cast(), rhs.as_ptr().cast()) };
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    if std::arch::is_aarch64_feature_detected!("neon") {

        return unsafe { hamming_distance_1280_neon(lhs.as_ptr().cast(), rhs.as_ptr().cast()) };
    }

    hamming_distance_u64_scalar(lhs, rhs)
}

/// Unrolled scalar popcount fallback over the packed words (also the wasm32
/// path). Kept as a separate function so it can be unit-tested directly: on
/// most CI machines the AVX2/NEON kernels mask this path, and the lane-skip
/// hazard it guards against is exactly what
/// `test_bhvec_hamming_scalar_fallback_matches_oracle` exercises.
pub(crate) fn hamming_distance_u64_scalar(lhs: &[u64; 160], rhs: &[u64; 160]) -> u32 {
    let mut distances = [0; 4];
    for i in (0..160).step_by(4) {
        distances[0] += (lhs[i] ^ rhs[i]).count_ones();
        distances[1] += (lhs[i + 1] ^ rhs[i + 1]).count_ones();
        distances[2] += (lhs[i + 2] ^ rhs[i + 2]).count_ones();
        distances[3] += (lhs[i + 3] ^ rhs[i + 3]).count_ones();
    }
    distances.into_iter().sum()
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
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

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// NEON XOR kernel over packed `[u64; 160]` words.
///
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn xor_simd_u64_neon(lhs: &[u64; 160], rhs: &[u64; 160]) -> [u64; 160] {
    use std::arch::aarch64::{veorq_u8, vld1q_u8, vst1q_u8};
    let mut res = std::mem::MaybeUninit::<[u64; 160]>::uninit();
    let res_ptr = res.as_mut_ptr().cast::<u8>();
    for i in (0..160).step_by(2) {



        unsafe {
            let l = vld1q_u8(lhs.as_ptr().add(i).cast());
            let r = vld1q_u8(rhs.as_ptr().add(i).cast());
            vst1q_u8(res_ptr.add(i * 8), veorq_u8(l, r));
        }
    }

    unsafe { res.assume_init() }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// AVX2 XOR kernel over packed `[u64; 160]` words.
///
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn xor_simd_u64_avx2(lhs: &[u64; 160], rhs: &[u64; 160]) -> [u64; 160] {
    let mut res = std::mem::MaybeUninit::<[u64; 160]>::uninit();
    let res_ptr = res.as_mut_ptr().cast::<u8>();
    for i in (0..160).step_by(4) {



        unsafe {
            let l = _mm256_loadu_si256(lhs.as_ptr().add(i).cast());
            let r = _mm256_loadu_si256(rhs.as_ptr().add(i).cast());
            _mm256_storeu_si256(res_ptr.add(i * 8).cast(), _mm256_xor_si256(l, r));
        }
    }

    unsafe { res.assume_init() }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
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
/// AVX2 popcount kernel over 1,280 raw bytes (40 unaligned 32-byte loads).
///
/// # SAFETY
/// `lhs` and `rhs` must each point to at least 1,280 readable bytes, and the
/// caller must ensure AVX2 is supported.
unsafe fn hamming_distance_1280_avx2(lhs: *const u8, rhs: *const u8) -> u32 {
    const LOADS_PER_FLUSH: usize = 20;
    const UNROLL_FACTOR: usize = 2;



    const _: () = assert!(80 % (LOADS_PER_FLUSH * 2) == 0);
    const _: () = assert!(LOADS_PER_FLUSH % (UNROLL_FACTOR * 2) == 0);

    let lookup = _mm256_setr_epi8(
        0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3,
        3, 4,
    );
    let low_mask = _mm256_set1_epi8(0x0f);
    let mut acc = _mm256_setzero_si256();
    let zero = _mm256_setzero_si256();





    for i in (0..80).step_by(LOADS_PER_FLUSH * 2) {
        let mut acc_8_low = _mm256_setzero_si256();
        let mut acc_8_high = _mm256_setzero_si256();
        for j in (0..LOADS_PER_FLUSH * 2).step_by(UNROLL_FACTOR * 2) {
            let idx0 = i + j;
            let idx1 = idx0 + 2;



            unsafe {
                let x0 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.add(idx0 * 16).cast()),
                    _mm256_loadu_si256(rhs.add(idx0 * 16).cast()),
                );
                let x1 = _mm256_xor_si256(
                    _mm256_loadu_si256(lhs.add(idx1 * 16).cast()),
                    _mm256_loadu_si256(rhs.add(idx1 * 16).cast()),
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

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// Hamming distance over `[u128; 80]` words, delegating to the shared raw-bytes
/// AVX2 popcount kernel.
///
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn hamming_distance_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {


    unsafe { hamming_distance_1280_avx2(lhs.as_ptr().cast(), rhs.as_ptr().cast()) }
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
/// NEON popcount kernel over 1,280 raw bytes.
///
/// # SAFETY
/// `lhs` and `rhs` must each point to at least 1,280 readable bytes, and the
/// caller must ensure NEON is supported.
unsafe fn hamming_distance_1280_neon(lhs: *const u8, rhs: *const u8) -> u32 {
    use std::arch::aarch64::{
        vaddlvq_u16, vaddq_u8, vaddq_u16, vcntq_u8, vdupq_n_u8, vdupq_n_u16, veorq_u8, vld1q_u8,
        vpaddlq_u8,
    };
    const BATCH_SIZE: usize = 10;
    const WORDS_PER_BATCH: usize = BATCH_SIZE * 2;

    const _: () = assert!(80 % WORDS_PER_BATCH == 0);

    let mut acc = vdupq_n_u16(0);

    for i in (0..80).step_by(WORDS_PER_BATCH) {






        let mut acc_8 = vdupq_n_u8(0);
        for j in 0..BATCH_SIZE {
            let idx = i + j * 2;


            unsafe {
                let l0 = vld1q_u8(lhs.add(idx * 16));
                let r0 = vld1q_u8(rhs.add(idx * 16));
                let x0 = veorq_u8(l0, r0);
                let c0 = vcntq_u8(x0);
                acc_8 = vaddq_u8(acc_8, c0);

                let l1 = vld1q_u8(lhs.add((idx + 1) * 16));
                let r1 = vld1q_u8(rhs.add((idx + 1) * 16));
                let x1 = veorq_u8(l1, r1);
                let c1 = vcntq_u8(x1);
                acc_8 = vaddq_u8(acc_8, c1);
            }
        }
        acc = vaddq_u16(acc, vpaddlq_u8(acc_8));
    }
    vaddlvq_u16(acc) as u32
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// Hamming distance over `[u128; 80]` words, delegating to the shared raw-bytes
/// NEON popcount kernel.
///
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn hamming_distance_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> u32 {


    unsafe { hamming_distance_1280_neon(lhs.as_ptr().cast(), rhs.as_ptr().cast()) }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    include!("hyperdim_simd_tests.rs");
}
