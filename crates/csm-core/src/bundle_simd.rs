#![allow(clippy::needless_range_loop)]
//! SIMD-optimized operations for BundleAccumulator.

/// AVX2-optimized bit-packing for bundle finalize.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn finalize_simd_avx2(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
    use std::arch::x86_64::{
        _mm256_castsi256_ps, _mm256_cmpgt_epi32, _mm256_loadu_si256, _mm256_movemask_ps,
        _mm256_set1_epi32,
    };
    let mut data = [0u128; 80];
    let threshold_vec = _mm256_set1_epi32(threshold);

    for i in 0..80 {
        let offset = i * 128;
        let mut word_low = 0u64;
        let mut word_high = 0u64;
        for j in 0..8 {
            // SAFETY: counts is [i32; 10240]. offset + j * 8 + 8 is within bounds.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + j * 8);
                let chunk = _mm256_loadu_si256(ptr.cast());
                let mask = _mm256_cmpgt_epi32(chunk, threshold_vec);
                _mm256_movemask_ps(_mm256_castsi256_ps(mask)) as u64
            };
            word_low |= packed << (j * 8);
        }
        for j in 0..8 {
            // SAFETY: offset + 64 + j * 8 + 8 is within bounds.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + 64 + j * 8);
                let chunk = _mm256_loadu_si256(ptr.cast());
                let mask = _mm256_cmpgt_epi32(chunk, threshold_vec);
                _mm256_movemask_ps(_mm256_castsi256_ps(mask)) as u64
            };
            word_high |= packed << (j * 8);
        }
        data[i] = (word_low as u128) | ((word_high as u128) << 64);
    }
    data
}

/// ARM NEON-optimized bit-packing for bundle finalize.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn finalize_simd_neon(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
    use std::arch::aarch64::{vaddvq_u32, vandq_u32, vcgtq_s32, vdupq_n_s32, vld1q_s32};
    let mut data = [0u128; 80];
    // SAFETY: weights is [u32; 4], which is 16 bytes. vld1q_u32 loads 16 bytes.
    let weights = unsafe {
        let w = [1u32, 2, 4, 8];
        std::arch::aarch64::vld1q_u32(w.as_ptr())
    };

    for i in 0..80 {
        let offset = i * 128;
        let mut word_low = 0u64;
        let mut word_high = 0u64;
        for j in 0..16 {
            // SAFETY: counts is [i32; 10240]. offset + j * 4 + 4 is within bounds.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + j * 4);
                let chunk = vld1q_s32(ptr);
                let mask = vcgtq_s32(chunk, vdupq_n_s32(threshold));
                let weighted = vandq_u32(mask, weights);
                vaddvq_u32(weighted) as u64
            };
            word_low |= packed << (j * 4);
        }
        for j in 0..16 {
            // SAFETY: offset + 64 + j * 4 + 4 is within bounds.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + 64 + j * 4);
                let chunk = vld1q_s32(ptr);
                let mask = vcgtq_s32(chunk, vdupq_n_s32(threshold));
                let weighted = vandq_u32(mask, weights);
                vaddvq_u32(weighted) as u64
            };
            word_high |= packed << (j * 4);
        }
        data[i] = (word_low as u128) | ((word_high as u128) << 64);
    }
    data
}

/// AVX2-optimized count accumulation for bundle add.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn update_counts_simd_avx2(
    counts: &mut [i32; 10240],
    hv: &[u128; 80],
    sign: i32,
) {
    use std::arch::x86_64::{
        _mm256_add_epi32, _mm256_and_si256, _mm256_cmpeq_epi32, _mm256_loadu_si256,
        _mm256_set1_epi32, _mm256_set_epi32, _mm256_storeu_si256,
    };

    let sign_vec = _mm256_set1_epi32(sign);
    // Bit selector for expanding bits to i32 lanes.
    // lane 0: 2^0, lane 1: 2^1, ..., lane 7: 2^7
    let bit_selector = _mm256_set_epi32(128, 64, 32, 16, 8, 4, 2, 1);

    for i in 0..80 {
        let word_ptr = &hv[i] as *const u128 as *const u8;
        // SAFETY: counts is [i32; 10240], i * 128 is within bounds.
        let counts_ptr = unsafe { counts.as_mut_ptr().add(i * 128) };

        for j in 0..16 {
            // SAFETY: hv[i] is u128 (16 bytes), j is 0..16.
            let byte = unsafe { *word_ptr.add(j) };
            if byte == 0 {
                continue;
            }

            // Expand 8 bits to 8 x i32 masks (0 or -1).
            // Process: broadcast byte -> and with bit_selector -> compare equal to bit_selector.
            let byte_vec = _mm256_set1_epi32(i32::from(byte));
            let mask = _mm256_cmpeq_epi32(_mm256_and_si256(byte_vec, bit_selector), bit_selector);
            let increment = _mm256_and_si256(mask, sign_vec);

            // SIMD add the increment (sign or 0) to the counts array.
            // SAFETY: counts_ptr + j * 8 + 8 is within bounds.
            let target_ptr = unsafe { counts_ptr.add(j * 8) };
            let current_counts = unsafe { _mm256_loadu_si256(target_ptr.cast()) };
            let new_counts = _mm256_add_epi32(current_counts, increment);
            unsafe { _mm256_storeu_si256(target_ptr.cast(), new_counts) };
        }
    }
}

/// ARM NEON-optimized count accumulation for bundle add.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn update_counts_simd_neon(
    counts: &mut [i32; 10240],
    hv: &[u128; 80],
    sign: i32,
) {
    for i in 0..80 {
        let word_ptr = &hv[i] as *const u128 as *const u8;
        // SAFETY: counts is [i32; 10240], i * 128 is within bounds.
        let counts_ptr = unsafe { counts.as_mut_ptr().add(i * 128) };

        for j in 0..16 {
            // SAFETY: hv[i] is u128 (16 bytes), j is 0..16.
            let byte = unsafe { *word_ptr.add(j) } as i32;
            if byte == 0 {
                continue;
            }

            for k in 0..8 {
                if (byte & (1 << k)) != 0 {
                    // SAFETY: counts_ptr + j * 8 + k is within bounds.
                    unsafe { *counts_ptr.add(j * 8 + k) += sign };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::hyperdim::HVec10240;
    use rand::Rng;

    fn finalize_scalar(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
        let mut data = [0u128; 80];
        for i in 0..80 {
            let offset = i * 128;
            for j in 0..128 {
                if counts[offset + j] > threshold {
                    data[i] |= 1u128 << j;
                }
            }
        }
        data
    }

    fn make_test_counts(seed: u64) -> [i32; 10240] {
        use rand::RngExt;
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut counts = [0i32; 10240];
        for i in 0..10240 {
            counts[i] = rng.random_range(-10..10);
        }
        counts
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn test_finalize_simd_avx2_consistency() {
        if is_x86_feature_detected!("avx2") {
            for seed in 0..10 {
                let counts = make_test_counts(seed);
                for threshold in [-2, -1, 0, 1, 2] {
                    let scalar = finalize_scalar(&counts, threshold);
                    // SAFETY: AVX2 is checked above.
                    let simd = unsafe { finalize_simd_avx2(&counts, threshold) };
                    assert_eq!(simd, scalar);
                }
            }
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    fn test_finalize_simd_neon_consistency() {
        for seed in 0..10 {
            let counts = make_test_counts(seed);
            for threshold in [-2, -1, 0, 1, 2] {
                let scalar = finalize_scalar(&counts, threshold);
                // SAFETY: NEON is always available on aarch64.
                let simd = unsafe { finalize_simd_neon(&counts, threshold) };
                assert_eq!(simd, scalar);
            }
        }
    }

    fn update_counts_scalar(counts: &mut [i32; 10240], hv: &[u128; 80], sign: i32) {
        for i in 0..80 {
            let mut val = hv[i];
            let offset = i * 128;
            for j in 0..128 {
                if (val & 1) != 0 {
                    counts[offset + j] += sign;
                }
                val >>= 1;
            }
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    fn test_update_counts_simd_avx2_consistency() {
        if is_x86_feature_detected!("avx2") {
            let mut counts_scalar = [0i32; 10240];
            let mut counts_simd = [0i32; 10240];
            let mut hvs = Vec::new();
            for i in 0..10 {
                hvs.push(HVec10240::new_seeded(i).data);
            }
            for hv in &hvs {
                update_counts_scalar(&mut counts_scalar, hv, 1);
                // SAFETY: AVX2 is checked above.
                unsafe { update_counts_simd_avx2(&mut counts_simd, hv, 1) };
            }
            assert_eq!(counts_scalar, counts_simd);
            for hv in &hvs {
                update_counts_scalar(&mut counts_scalar, hv, -1);
                // SAFETY: AVX2 is checked above.
                unsafe { update_counts_simd_avx2(&mut counts_simd, hv, -1) };
            }
            assert_eq!(counts_scalar, counts_simd);
        }
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    fn test_update_counts_simd_neon_consistency() {
        let mut counts_scalar = [0i32; 10240];
        let mut counts_simd = [0i32; 10240];
        let mut hvs = Vec::new();
        for i in 0..10 {
            hvs.push(HVec10240::new_seeded(i).data);
        }
        for hv in &hvs {
            update_counts_scalar(&mut counts_scalar, hv, 1);
            // SAFETY: NEON is always available on aarch64.
            unsafe { update_counts_simd_neon(&mut counts_simd, hv, 1) };
        }
        assert_eq!(counts_scalar, counts_simd);
        for hv in &hvs {
            update_counts_scalar(&mut counts_scalar, hv, -1);
            // SAFETY: NEON is always available on aarch64.
            unsafe { update_counts_simd_neon(&mut counts_simd, hv, -1) };
        }
        assert_eq!(counts_scalar, counts_simd);
    }
}
