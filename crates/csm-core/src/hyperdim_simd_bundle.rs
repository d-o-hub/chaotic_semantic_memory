//! SIMD-optimized hypervector bundle operations.

/// AVX2-optimized bit-sliced bundling for a single 256-bit block (2 words).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
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
        // SAFETY: v.data is [u128; 80]. word_idx + 2 is within bounds.
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
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn bundle_block_avx2(
    vectors: &[crate::hyperdim::HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::x86_64::_mm256_storeu_si256;
    let mut out = [0u128; 80];
    for i in (0..80).step_by(2) {
        // SAFETY: AVX2 is detected at runtime.
        let res = unsafe { bundle_block_avx2_single(vectors, i, threshold, num_planes) };
        // SAFETY: out is [u128; 80]. i + 2 is within bounds.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// ARM NEON-optimized bit-sliced bundling for a single 128-bit block (1 word).
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
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
        // SAFETY: v.data is [u128; 80]. word_idx is within bounds.
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
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn bundle_block_neon(
    vectors: &[crate::hyperdim::HVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u128; 80] {
    use std::arch::aarch64::vst1q_u8;
    let mut out = [0u128; 80];
    for i in 0..80 {
        // SAFETY: NEON is supported.
        let res = unsafe { bundle_block_neon_single(vectors, i, threshold, num_planes) };
        // SAFETY: out is [u128; 80]. i is within bounds.
        unsafe { vst1q_u8(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// AVX2-optimized bit-sliced bundling for a single 256-bit block (4 u64 words) of BHVec10240.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn bundle_block_avx2_single_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    word_idx: usize,
    threshold: usize,
    num_planes: usize,
) -> std::arch::x86_64::__m256i {
    use std::arch::x86_64::{
        _mm256_and_si256, _mm256_andnot_si256, _mm256_loadu_si256, _mm256_or_si256,
        _mm256_set1_epi64x, _mm256_setzero_si256, _mm256_testz_si256, _mm256_xor_si256,
    };

    // Stack allocation note: 'planes' is a 64-entry array of 256-bit registers (2 KB on stack),
    // which is safely within normal stack limits and called in shallow recursion paths.
    let mut planes = [_mm256_setzero_si256(); 64];
    for v in vectors {
        // SAFETY: v.bits is [u64; 160]. word_idx + 4 is within bounds.
        let mut carry = unsafe { _mm256_loadu_si256(v.bits.as_ptr().add(word_idx).cast()) };
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
#[inline]
/// # SAFETY
/// Caller must ensure AVX2 is supported.
pub(crate) unsafe fn bundle_block_avx2_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u64; 160] {
    use std::arch::x86_64::_mm256_storeu_si256;
    let mut out = [0u64; 160];
    for i in (0..160).step_by(4) {
        // SAFETY: AVX2 is supported.
        let res = unsafe { bundle_block_avx2_single_bhvec(vectors, i, threshold, num_planes) };
        // SAFETY: out is [u64; 160]. i + 4 is within bounds.
        unsafe { _mm256_storeu_si256(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// ARM NEON-optimized bit-sliced bundling for a single 128-bit block (2 u64 words) of BHVec10240.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn bundle_block_neon_single_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    word_idx: usize,
    threshold: usize,
    num_planes: usize,
) -> std::arch::aarch64::uint8x16_t {
    use std::arch::aarch64::{
        vandq_u8, vbicq_u8, vdupq_n_u8, veorq_u8, vgetq_lane_u64, vld1q_u8, vorrq_u8,
        vreinterpretq_u64_u8,
    };

    // Stack allocation note: 'planes' is a 64-entry array of 128-bit registers (1 KB on stack),
    // which is safely within normal stack limits and called in shallow recursion paths.
    let mut planes = [vdupq_n_u8(0); 64];
    for v in vectors {
        // SAFETY: v.bits is an array of [u64; 160]. word_idx is a u64 word index.
        // vst1q_u8/vld1q_u8 load/store 16 bytes, which corresponds exactly to 2 u64 words.
        // Since word_idx <= 158 and word_idx is even, word_idx + 2 is strictly <= 160,
        // so accessing starting at word_idx is completely within bounds of the 160-word bits array.
        let mut carry = unsafe { vld1q_u8(v.bits.as_ptr().add(word_idx).cast()) };
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
#[inline]
/// # SAFETY
/// Caller must ensure NEON is supported.
pub(crate) unsafe fn bundle_block_neon_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u64; 160] {
    use std::arch::aarch64::vst1q_u8;
    let mut out = [0u64; 160];
    for i in (0..160).step_by(2) {
        // SAFETY: NEON is supported.
        let res = unsafe { bundle_block_neon_single_bhvec(vectors, i, threshold, num_planes) };
        // SAFETY: out is [u64; 160]. i + 2 is within bounds.
        unsafe { vst1q_u8(out.as_mut_ptr().add(i).cast(), res) };
    }
    out
}

/// Safe wrapper for sequential AVX2 bundling of BHVec10240.
/// Returns None if AVX2 is not supported.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
pub(crate) fn bundle_avx2_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> Option<[u64; 160]> {
    #[allow(clippy::eq_op)]
    const _: () = assert!(160 % 4 == 0);
    if is_x86_feature_detected!("avx2") {
        // SAFETY: AVX2 is detected at runtime.
        Some(unsafe { bundle_block_avx2_bhvec(vectors, threshold, num_planes) })
    } else {
        None
    }
}

/// Safe wrapper for parallel AVX2 bundling of BHVec10240.
/// Returns None if AVX2 is not supported.
#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "parallel",
    target_arch = "x86_64"
))]
pub(crate) fn bundle_parallel_avx2_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> Option<[u64; 160]> {
    #[allow(clippy::eq_op)]
    const _: () = assert!(160 % 4 == 0);
    if is_x86_feature_detected!("avx2") {
        use rayon::prelude::*;
        let mut bits = [0u64; 160];
        bits.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
            // SAFETY: AVX2 is supported. Pointers are valid.
            let res =
                unsafe { bundle_block_avx2_single_bhvec(vectors, i * 4, threshold, num_planes) };
            // SAFETY: chunk length is 4 (32 bytes), matching AVX2 256-bit block size.
            unsafe {
                std::arch::x86_64::_mm256_storeu_si256(chunk.as_mut_ptr().cast(), res);
            }
        });
        Some(bits)
    } else {
        None
    }
}

/// Safe wrapper for sequential NEON bundling of BHVec10240.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
pub(crate) fn bundle_neon_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u64; 160] {
    #[allow(clippy::eq_op)]
    const _: () = assert!(160 % 2 == 0);
    // SAFETY: NEON is always supported on aarch64.
    unsafe { bundle_block_neon_bhvec(vectors, threshold, num_planes) }
}

/// Safe wrapper for parallel NEON bundling of BHVec10240.
#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "parallel",
    target_arch = "aarch64"
))]
pub(crate) fn bundle_parallel_neon_bhvec(
    vectors: &[&crate::hyperdim::BHVec10240],
    threshold: usize,
    num_planes: usize,
) -> [u64; 160] {
    #[allow(clippy::eq_op)]
    const _: () = assert!(160 % 2 == 0);
    use rayon::prelude::*;
    let mut bits = [0u64; 160];
    bits.par_chunks_mut(2).enumerate().for_each(|(i, chunk)| {
        // SAFETY: NEON is supported on aarch64. Pointers are valid.
        let res = unsafe { bundle_block_neon_single_bhvec(vectors, i * 2, threshold, num_planes) };
        // SAFETY: chunk length is 2 (16 bytes), matching NEON 128-bit block size.
        unsafe {
            std::arch::aarch64::vst1q_u8(chunk.as_mut_ptr().cast(), res);
        }
    });
    bits
}

#[cfg(test)]
#[path = "hyperdim_simd_bundle/simd_bundle_tests.rs"]
mod tests;
