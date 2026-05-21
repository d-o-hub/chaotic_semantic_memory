//! Tests for SIMD-optimized hypervector operations.
//!
//! Unit tests for platform-specific SIMD implementations:
//! - x86/x86_64: SSE (128-bit) and AVX2 (256-bit) bind, AND, Hamming distance
//! - aarch64: NEON (128-bit) bind, AND, Hamming distance
//!
//! Run as part of the crate's unit test tree via `#[path]` attribute
//! from `hyperdim_simd.rs`, which gives access to `pub(crate)` functions.

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
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_correctness() {
    if std::arch::is_x86_feature_detected!("avx2") {
        let (lhs, rhs) = make_test_vectors();
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        assert_eq!(simd, scalar);
        // Test with random vectors
        use crate::hyperdim::HVec10240;
        for i in 0..10 {
            let v1 = HVec10240::new_seeded(i);
            let v2 = HVec10240::new_seeded(i + 100);
            let scalar_r = hamming_distance_optimized(&v1.data, &v2.data);
            let simd_r = unsafe { hamming_distance_simd_avx2(&v1.data, &v2.data) };
            assert_eq!(simd_r, scalar_r, "Failed on iteration {}", i);
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_all_ones_vs_zeros() {
    if std::arch::is_x86_feature_detected!("avx2") {
        let lhs = [0xFFFFFFFFFFFFFFFF_FFFFFFFFFFFFFFFFu128; 80];
        let rhs = [0u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
        assert_eq!(simd, 10240);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_identical() {
    if std::arch::is_x86_feature_detected!("avx2") {
        let v = [0x123456789ABCDEF_0FEDCBA987654321u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&v, &v) };
        assert_eq!(simd, 0);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_complements() {
    if std::arch::is_x86_feature_detected!("avx2") {
        let lhs = [0xAAAAAAAAAAAAAAAA_AAAAAAAAAAAAAAAAu128; 80];
        let rhs = [0x5555555555555555_5555555555555555u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
        assert_eq!(simd, 10240);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_single_bit() {
    if std::arch::is_x86_feature_detected!("avx2") {
        // Set one bit per u128 word — 80 bits total
        let lhs: [u128; 80] = std::array::from_fn(|i| 1u128 << (i % 128));
        let rhs = [0u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
        assert_eq!(simd, 80);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_alternating_pattern() {
    if std::arch::is_x86_feature_detected!("avx2") {
        // Alternating 0xAAAA... and 0x5555... per word
        let lhs: [u128; 80] = std::array::from_fn(|i| {
            if i % 2 == 0 {
                0xAAAAAAAAAAAAAAAA_AAAAAAAAAAAAAAAAu128
            } else {
                0x5555555555555555_5555555555555555u128
            }
        });
        let rhs: [u128; 80] = std::array::from_fn(|i| {
            if i % 2 == 0 {
                0x5555555555555555_5555555555555555u128
            } else {
                0xAAAAAAAAAAAAAAAA_AAAAAAAAAAAAAAAAu128
            }
        });
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_random_100() {
    if std::arch::is_x86_feature_detected!("avx2") {
        use crate::hyperdim::HVec10240;
        // Expanded random test: 100 seeded vector pairs
        for i in 0..100 {
            let v1 = HVec10240::new_seeded(i);
            let v2 = HVec10240::new_seeded(i + 1000);
            let scalar = hamming_distance_optimized(&v1.data, &v2.data);
            let simd = unsafe { hamming_distance_simd_avx2(&v1.data, &v2.data) };
            assert_eq!(simd, scalar, "Failed on iteration {}", i);
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_sparse_pattern() {
    if std::arch::is_x86_feature_detected!("avx2") {
        // Sparse: only 1 bit set across entire 10240-bit vector
        // This tests edge cases where popcounts don't align with unroll factor (4)
        let mut lhs = [0u128; 80];
        lhs[42] = 1u128 << 73; // bit at position 42*128 + 73 = 5449
        let rhs = [0u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
        assert_eq!(simd, 1);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn hamming_distance_simd_avx2_mixed_popcounts() {
    if std::arch::is_x86_feature_detected!("avx2") {
        // Each word has a different number of set bits (mod 128)
        // This stresses the dual-accumulator path with varied popcounts
        let lhs: [u128; 80] = std::array::from_fn(|i| {
            let bits = (i % 128) + 1;
            if bits >= 128 { 0 } else { (1u128 << bits) - 1 }
        });
        let rhs = [0u128; 80];
        let simd = unsafe { hamming_distance_simd_avx2(&lhs, &rhs) };
        let scalar = hamming_distance_optimized(&lhs, &rhs);
        assert_eq!(simd, scalar);
    }
}
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn bundle_block_avx2_correctness() {
    if std::arch::is_x86_feature_detected!("avx2") {
        use crate::hyperdim::HVec10240;
        let vectors: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
        let threshold = vectors.len() / 2 + 1;
        let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
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
