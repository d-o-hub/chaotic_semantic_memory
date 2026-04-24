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
        unsafe {
            let a = _mm_loadu_si128((&lhs[i] as *const u128).cast::<__m128i>());
            let b = _mm_loadu_si128((&rhs[i] as *const u128).cast::<__m128i>());
            let x = _mm_xor_si128(a, b);
            _mm_storeu_si128((&mut out[i] as *mut u128).cast::<__m128i>(), x);
        }
    }
    out
}

/// AVX2-optimized bind (256-bit XOR, processes 2 words per instruction).
/// Uses runtime feature detection to dispatch when AVX2 is available.
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[inline]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn bind_simd_avx2(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::x86_64::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256, _mm256_xor_si256};

    let mut out = [0u128; 80];
    // Process pairs of u128s (32 bytes per AVX2 instruction)
    for i in (0..80).step_by(2) {
        // SAFETY: AVX2 requires 32-byte alignment; u128 array is 16-byte aligned.
        // Using unaligned loads handles this safely. Pointer arithmetic is within bounds.
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
pub(crate) unsafe fn bind_simd_neon(lhs: &[u128; 80], rhs: &[u128; 80]) -> [u128; 80] {
    use std::arch::aarch64::{uint64x2_t, veorq_u64, vld1q_u64, vst1q_u64};

    let mut out = [0u128; 80];
    for i in 0..80 {
        // SAFETY: u128 is 16-byte aligned; we reinterpret as two 64-bit halves.
        // Pointer arithmetic is within bounds of the [u128; 80] arrays.
        let lhs_ptr = lhs.as_ptr().add(i) as *const uint64x2_t;
        let rhs_ptr = rhs.as_ptr().add(i) as *const uint64x2_t;
        let out_ptr = out.as_mut_ptr().add(i) as *mut uint64x2_t;

        let a = vld1q_u64(lhs_ptr);
        let b = vld1q_u64(rhs_ptr);
        let x = veorq_u64(a, b);
        vst1q_u64(out_ptr, x);
    }
    out
}
