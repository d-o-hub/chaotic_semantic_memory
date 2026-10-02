use super::*;
use crate::hyperdim_binary::BHVec10240;

#[test]
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
fn and_simd_avx2_correctness() {
    if is_x86_feature_detected!("avx2") {
        let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
        // SAFETY: AVX2 support is checked above.
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
        // SAFETY: AVX2 support is checked above.
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
        // SAFETY: AVX2 support is checked above.
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
        // SAFETY: AVX2 support is checked above.
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
    assert_eq!(hamming_distance_optimized(&lhs, &rhs), 2);
}

/// Operands whose non-zero lanes sit in the head, the last full vector chunk
/// (i = 156) and the final word (i = 159) — the positions a wrong `step_by`
/// or a short store would corrupt first.
fn xor_tail_fixture() -> ([u64; 160], [u64; 160]) {
    let mut lhs = [0x0123_4567_89AB_CDEFu64; 160];
    let mut rhs = [0xFEDC_BA98_7654_3210u64; 160];
    lhs[0] = 0;
    rhs[0] = u64::MAX;
    lhs[156] = 0xAAAA_AAAA_AAAA_AAAA;
    rhs[156] = 0x5555_5555_5555_5555;
    lhs[159] = 1;
    rhs[159] = u64::MAX;
    (lhs, rhs)
}

#[test]
fn xor_u64_dispatched_matches_scalar() {
    // Runs on every target: the dispatched path (AVX2/NEON when the host
    // supports it, the scalar fallback otherwise) must equal the scalar
    // oracle word for word.
    let (lhs, rhs) = xor_tail_fixture();
    assert_eq!(xor_u64(&lhs, &rhs), xor_u64_scalar(&lhs, &rhs));
}

#[test]
fn xor_u64_scalar_is_involutive() {
    let (lhs, rhs) = xor_tail_fixture();
    let bound = xor_u64_scalar(&lhs, &rhs);
    assert_eq!(xor_u64_scalar(&bound, &rhs), lhs);
    assert_eq!(xor_u64_scalar(&lhs, &lhs), [0u64; 160]);
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
fn xor_simd_u64_avx2_matches_scalar() {
    if is_x86_feature_detected!("avx2") {
        let (lhs, rhs) = xor_tail_fixture();
        // SAFETY: AVX2 support is checked above; both arrays are 1,280 bytes.
        let res = unsafe { xor_simd_u64_avx2(&lhs, &rhs) };
        assert_eq!(res, xor_u64_scalar(&lhs, &rhs));
    }
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
fn xor_simd_u64_neon_matches_scalar() {
    if std::arch::is_aarch64_feature_detected!("neon") {
        let (lhs, rhs) = xor_tail_fixture();
        // SAFETY: NEON support is checked above; both arrays are 1,280 bytes.
        let res = unsafe { xor_simd_u64_neon(&lhs, &rhs) };
        assert_eq!(res, xor_u64_scalar(&lhs, &rhs));
    }
}

#[test]
fn bhvec_xor_matches_wordwise_scalar() {
    let a = BHVec10240::random();
    let b = BHVec10240::random();
    let bound = a.xor(&b);
    for i in 0..160 {
        assert_eq!(bound.bits[i], a.bits[i] ^ b.bits[i]);
    }
    assert_eq!(a.xor(&a).bits, [0u64; 160]);
}

#[test]
fn bhvec_xor_agrees_with_hvec_bind() {
    // The `[u64; 160]` kernel and the `[u128; 80]` binding kernels are the
    // same 1,280-byte XOR; the two layouts must not drift apart.
    let a = BHVec10240::random();
    let b = BHVec10240::random();
    assert_eq!(a.xor(&b).to_hvec(), a.to_hvec().bind(&b.to_hvec()));
}
