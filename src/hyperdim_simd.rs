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
    // SAFETY: Transmuting to u64 pointers is safe because u128 is 16-byte aligned
    // and u64 is 8-byte aligned. Array size 80 * u128 is 160 * u64.
    unsafe {
        let lptr = lhs.as_ptr() as *const u64;
        let rptr = rhs.as_ptr() as *const u64;

        // Use multiple independent accumulators to break the serial dependency chain.
        // This allows the CPU to utilize multiple execution ports for ILP.
        let mut s0 = 0;
        let mut s1 = 0;
        let mut s2 = 0;
        let mut s3 = 0;

        // Unroll for better port utilization and pipelining
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
        // SAFETY: `u128` is 16-byte aligned, matching `__m128i` requirements.
        // Array indexing is within bounds (0..80).
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
        // SAFETY: `u128` is 16-byte aligned. Array indexing is within bounds.
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
        // SAFETY: AVX2 intrinsics are safe when feature is detected by caller.
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
        // SAFETY: NEON is always available on aarch64.
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
/// Uses runtime feature detection to dispatch when AVX2 is available.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
/// # Safety
/// This function is unsafe because it uses AVX2 intrinsics. The caller must ensure that
/// AVX2 is supported by the CPU at runtime.
pub(crate) unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::x86_64::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256, _mm256_xor_si256};

    let mut out = [0u128; 80];
    // Process pairs of u128s (32 bytes per AVX2 instruction)
    for i in (0..80).step_by(2) {
        // SAFETY: AVX2 requires 32-byte alignment; u128 array is 16-byte aligned.
        // Using unaligned loads (_mm256_loadu_si256) handles this safely.
        // Pointer arithmetic and array access are within bounds (80 elements).
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
/// Uses uint64x2_t to process each 128-bit word as two 64-bit halves.
/// NEON is always available on aarch64.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
/// # Safety
/// This function is unsafe because it uses NEON intrinsics. The caller must ensure that
/// NEON is supported by the CPU (always true for aarch64).
pub(crate) unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{veorq_u64, vld1q_u64, vst1q_u64};

    let mut out = [0u128; 80];
    for i in 0..80 {
        // SAFETY: u128 is 16-byte aligned; we cast to *const u64 which is correct
        // for vld1q_u64. The pointer arithmetic is within bounds (80 words).
        // All unsafe operations are in an explicit unsafe block as required by
        // #[target_feature(enable = "neon")].
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

/// AVX2-optimized bit-packing for bundle finalize.
///
/// Processes 8 bit-counts at once using 256-bit registers.
/// Compares each count against zero and packs the results into an 8-bit mask.
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

        // Process 128 bits in 16 chunks of 8 bits each
        // Lower 64 bits (8 chunks)
        for j in 0..8 {
            // SAFETY: Array indexing is within bounds (10240 counts).
            // AVX2 intrinsics are safe when feature is detected by caller.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + j * 8);
                let chunk = _mm256_loadu_si256(ptr.cast());
                let mask = _mm256_cmpgt_epi32(chunk, threshold_vec);
                // _mm256_movemask_ps treats each 32-bit lane as a float and takes the sign bit
                // Since our mask is all 1s (negative in float) or all 0s, this works perfectly.
                _mm256_movemask_ps(_mm256_castsi256_ps(mask)) as u64
            };
            word_low |= packed << (j * 8);
        }

        // Upper 64 bits (8 chunks)
        for j in 0..8 {
            // SAFETY: Array indexing is within bounds (10240 counts).
            // AVX2 intrinsics are safe when feature is detected by caller.
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
///
/// Processes 4 bit-counts at once using 128-bit registers.
/// Compares each count against zero and packs the results using bit-shifts and additions.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[inline]
#[target_feature(enable = "neon")]
pub(crate) unsafe fn finalize_simd_neon(counts: &[i32; 10240], threshold: i32) -> [u128; 80] {
    use std::arch::aarch64::{vaddvq_u32, vandq_u32, vcgtq_s32, vdupq_n_s32, vld1q_s32};

    let mut data = [0u128; 80];
    // Bit weights for packing 4 bits into a single u32 via vaddvq
    // SAFETY: vld1q_u32 is safe for loading these constants.
    let weights = unsafe {
        let w = [1u32, 2, 4, 8];
        std::arch::aarch64::vld1q_u32(w.as_ptr())
    };

    for i in 0..80 {
        let offset = i * 128;
        let mut word_low = 0u64;
        let mut word_high = 0u64;

        // Process 128 bits in 32 chunks of 4 bits each
        // Lower 64 bits (16 chunks)
        for j in 0..16 {
            // SAFETY: Array indexing is within bounds (10240 counts).
            // NEON is always available on aarch64.
            let packed = unsafe {
                let ptr = counts.as_ptr().add(offset + j * 4);
                let chunk = vld1q_s32(ptr);
                let mask = vcgtq_s32(chunk, vdupq_n_s32(threshold));
                // Apply weights to the mask (0xFFFFFFFF for set bits, 0 for clear)
                let weighted = vandq_u32(mask, weights);
                // Sum to pack the 4 bits into the bottom of a u32
                vaddvq_u32(weighted) as u64
            };
            word_low |= packed << (j * 4);
        }

        // Upper 64 bits (16 chunks)
        for j in 0..16 {
            // SAFETY: Array indexing is within bounds (10240 counts).
            // NEON is always available on aarch64.
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
}
