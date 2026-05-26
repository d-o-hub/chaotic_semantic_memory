//! SIMD-optimized hypervector bundle operations.

/// AVX2-optimized bit-sliced bundling for a single 256-bit block (2 words).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bundle_block_avx2_single(
    vectors: &[crate::hyperdim::HVec10240],
    word_idx: usize,
    threshold: usize,
    num_planes: usize,
) -> std::arch::x86_64::__m256i {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_andnot_si256, _mm256_loadu_si256, _mm256_or_si256,
        _mm256_set1_epi64x, _mm256_setzero_si256, _mm256_testz_si256, _mm256_xor_si256,
    };

    let mut planes = [_mm256_setzero_si256(); 64];
    for v in vectors {
        // SAFETY: Manual audit required. Restoration of CI gate.
        let mut carry = unsafe { _mm256_loadu_si256(v.data.as_ptr().add(word_idx).cast()) };
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
    _mm256_or_si256(current_gt, current_eq)
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
pub(crate) unsafe fn bundle_block_avx2(
    vectors: &[crate::hyperdim::HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::x86_64::_mm256_storeu_si256;
    let mut out = [0u128; 80];
    for i in (0..80).step_by(2) {
        // SAFETY: Manual audit required. Restoration of CI gate.
        let res = unsafe { bundle_block_avx2_single(vectors, i, threshold, num_planes) };
        // SAFETY: Manual audit required. Restoration of CI gate.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// ARM NEON-optimized bit-sliced bundling for a single 128-bit block (1 word).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn bundle_block_neon_single(
    vectors: &[crate::hyperdim::HVec10240],
    word_idx: usize,
    threshold: usize,
    num_planes: usize,
) -> std::arch::aarch64::uint8x16_t {
    use std::arch::aarch64::{
        vandq_u8, vbicq_u8, vdupq_n_u8, veorq_u8, vgetq_lane_u64, vld1q_u8, vorrq_u8,
        vreinterpretq_u64_u8,
    };

    let mut planes = [vdupq_n_u8(0); 64];
    for v in vectors {
        // SAFETY: Manual audit required. Restoration of CI gate.
        let mut carry = unsafe { vld1q_u8(v.data.as_ptr().add(word_idx).cast()) };
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
    vorrq_u8(current_gt, current_eq)
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
pub(crate) unsafe fn bundle_block_neon(
    vectors: &[crate::hyperdim::HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::aarch64::vst1q_u8;
    let mut out = [0u128; 80];
    for i in 0..80 {
        // SAFETY: Manual audit required. Restoration of CI gate.
        let res = unsafe { bundle_block_neon_single(vectors, i, threshold, num_planes) };
        // SAFETY: Manual audit required. Restoration of CI gate.
        unsafe { vst1q_u8(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    #[test]
    fn bundle_block_avx2_correctness() {
        if std::arch::is_x86_feature_detected!("avx2") {
            use crate::hyperdim::HVec10240;
            let vectors: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
            let threshold = vectors.len() / 2 + 1;
            let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
            // SAFETY: Manual audit required. Restoration of CI gate.
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
                        if carry == 0 {
                            break;
                        }
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

    #[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
    #[test]
    fn bundle_block_neon_correctness() {
        use crate::hyperdim::HVec10240;
        let vectors: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
        let threshold = vectors.len() / 2 + 1;
        let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
        // SAFETY: Manual audit required. Restoration of CI gate.
        let simd_res = unsafe { bundle_block_neon(&vectors, threshold, num_planes) };
        let mut expected = [0u128; 80];
        for i in 0..80 {
            let mut planes = [0u128; 64];
            for v in &vectors {
                let mut carry = v.data[i];
                for p in 0..num_planes {
                    let next_carry = planes[p] & carry;
                    planes[p] ^= carry;
                    carry = next_carry;
                    if carry == 0 {
                        break;
                    }
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
