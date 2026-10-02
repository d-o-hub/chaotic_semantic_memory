use super::*;

#[test]
#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
fn and_simd_avx2_correctness() {
    if is_x86_feature_detected!("avx2") {
        let lhs = [0x5555_5555_5555_5555_5555_5555_5555_5555u128; 80];
        let rhs = [0xAAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAA_AAAAu128; 80];
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
