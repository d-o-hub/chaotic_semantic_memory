//! SIMD-optimized operations for BundleAccumulator.

/// AVX2-optimized bit-packing for bundle finalize.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
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
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + j * 8);
                let chunk = _mm256_loadu_si256(ptr.cast());
                let mask = _mm256_cmpgt_epi32(chunk, threshold_vec);
                _mm256_movemask_ps(_mm256_castsi256_ps(mask)) as u64
            };
            word_low |= packed << (j * 8);
        }
        for j in 0..8 {
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
pub(crate) unsafe fn finalize_simd_neon(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
    use std::arch::aarch64::{vaddvq_u32, vandq_u32, vcgtq_s32, vdupq_n_s32, vld1q_s32};
    let mut data = [0u128; 80];
    let weights = unsafe {
        let w = [1u32, 2, 4, 8];
        std::arch::aarch64::vld1q_u32(w.as_ptr())
    };
    for i in 0..80 {
        let offset = i * 128;
        let mut word_low = 0u64;
        let mut word_high = 0u64;
        for j in 0..16 {
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

/// AVX2-optimized incremental count update.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn update_counts_simd_avx2(
    counts: &mut [i32; 10240],
    hv: &[u128; 80],
    sign: i32,
) {
    use std::arch::x86_64::{
        _mm256_add_epi32, _mm256_and_si256, _mm256_cmpeq_epi32, _mm256_loadu_si256,
        _mm256_set_epi32, _mm256_set1_epi32, _mm256_storeu_si256,
    };

    let sign_vec = _mm256_set1_epi32(sign);
    let masks = _mm256_set_epi32(0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01);

    for i in 0..80 {
        let word_ptr = &hv[i] as *const u128 as *const u8;
        let counts_ptr = unsafe { counts.as_mut_ptr().add(i * 128) };

        for j in 0..16 {
            let byte = unsafe { *word_ptr.add(j) };
            if byte == 0 {
                continue;
            }

            let v_byte = _mm256_set1_epi32(byte as i32);
            let v_and = _mm256_and_si256(v_byte, masks);
            let v_cmp = _mm256_cmpeq_epi32(v_and, masks);
            let inc = _mm256_and_si256(v_cmp, sign_vec);

            let target_ptr = unsafe { counts_ptr.add(j * 8) as *mut _ };
            let current = unsafe { _mm256_loadu_si256(target_ptr) };
            let updated = _mm256_add_epi32(current, inc);
            unsafe { _mm256_storeu_si256(target_ptr, updated) };
        }
    }
}

/// ARM NEON-optimized incremental count update.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn update_counts_simd_neon(
    counts: &mut [i32; 10240],
    hv: &[u128; 80],
    sign: i32,
) {
    use std::arch::aarch64::{
        vaddq_s32, vandq_s32, vceqq_s32, vdupq_n_s32, vld1q_s32, vreinterpretq_s32_u32, vst1q_s32,
    };

    let sign_vec = vdupq_n_s32(sign);
    let mask_vals = [0x01i32, 0x02, 0x04, 0x08];
    let masks_low = unsafe { vld1q_s32(mask_vals.as_ptr()) };
    let mask_vals_high = [0x10i32, 0x20, 0x40, 0x80];
    let masks_high = unsafe { vld1q_s32(mask_vals_high.as_ptr()) };

    for i in 0..80 {
        let word_ptr = &hv[i] as *const u128 as *const u8;
        let counts_ptr = unsafe { counts.as_mut_ptr().add(i * 128) };

        for j in 0..16 {
            let byte = unsafe { *word_ptr.add(j) } as i32;
            if byte == 0 {
                continue;
            }

            let v_byte = vdupq_n_s32(byte);

            // Lower 4 bits
            let v_and_l = vandq_s32(v_byte, masks_low);
            let v_cmp_l = vceqq_s32(v_and_l, masks_low);
            let inc_l = vandq_s32(vreinterpretq_s32_u32(v_cmp_l), sign_vec);

            let target_ptr_l = unsafe { counts_ptr.add(j * 8) };
            let current_l = unsafe { vld1q_s32(target_ptr_l) };
            unsafe { vst1q_s32(target_ptr_l as *mut _, vaddq_s32(current_l, inc_l)) };

            // Upper 4 bits
            let v_and_h = vandq_s32(v_byte, masks_high);
            let v_cmp_h = vceqq_s32(v_and_h, masks_high);
            let inc_h = vandq_s32(vreinterpretq_s32_u32(v_cmp_h), sign_vec);

            let target_ptr_h = unsafe { counts_ptr.add(j * 8 + 4) };
            let current_h = unsafe { vld1q_s32(target_ptr_h) };
            unsafe { vst1q_s32(target_ptr_h as *mut _, vaddq_s32(current_h, inc_h)) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyperdim::HVec10240;

    fn finalize_scalar(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
        let mut data = [0u128; 80];
        for (i, word) in data.iter_mut().enumerate() {
            let offset = i * 128;
            for j in 0..128 {
                if counts[offset + j] > threshold {
                    *word |= 1u128 << j;
                }
            }
        }
        data
    }

    fn make_test_counts(seed: u64) -> [i32; 10240] {
        use rand::{RngExt, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut counts = [0i32; 10240];
        for i in 0..10240 {
            counts[i] = rng.random_range(-10..10);
        }
        counts
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn test_finalize_simd_avx2_consistency() {
        if std::arch::is_x86_feature_detected!("avx2") {
            for seed in 0..10 {
                let counts = make_test_counts(seed);
                for threshold in [-2, -1, 0, 1, 2] {
                    let scalar = finalize_scalar(&counts, threshold);
                    let simd = unsafe { finalize_simd_avx2(&counts, threshold) };
                    assert_eq!(simd, scalar);
                }
            }
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    #[test]
    fn test_finalize_simd_neon_consistency() {
        for seed in 0..10 {
            let counts = make_test_counts(seed);
            for threshold in [-2, -1, 0, 1, 2] {
                let scalar = finalize_scalar(&counts, threshold);
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

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn test_update_counts_simd_avx2_consistency() {
        if std::arch::is_x86_feature_detected!("avx2") {
            let mut counts_scalar = [0i32; 10240];
            let mut counts_simd = [0i32; 10240];
            let mut hvs = Vec::new();
            for i in 0..10 {
                hvs.push(HVec10240::new_seeded(i).data);
            }
            for hv in &hvs {
                update_counts_scalar(&mut counts_scalar, hv, 1);
                unsafe { update_counts_simd_avx2(&mut counts_simd, hv, 1) };
            }
            assert_eq!(counts_scalar, counts_simd);
            for hv in &hvs {
                update_counts_scalar(&mut counts_scalar, hv, -1);
                unsafe { update_counts_simd_avx2(&mut counts_simd, hv, -1) };
            }
            assert_eq!(counts_scalar, counts_simd);
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    #[test]
    fn test_update_counts_simd_neon_consistency() {
        let mut counts_scalar = [0i32; 10240];
        let mut counts_simd = [0i32; 10240];
        let mut hvs = Vec::new();
        for i in 0..10 {
            hvs.push(HVec10240::new_seeded(i).data);
        }
        for hv in &hvs {
            update_counts_scalar(&mut counts_scalar, hv, 1);
            unsafe { update_counts_simd_neon(&mut counts_simd, hv, 1) };
        }
        assert_eq!(counts_scalar, counts_simd);
        for hv in &hvs {
            update_counts_scalar(&mut counts_scalar, hv, -1);
            unsafe { update_counts_simd_neon(&mut counts_simd, hv, -1) };
        }
        assert_eq!(counts_scalar, counts_simd);
    }
}
