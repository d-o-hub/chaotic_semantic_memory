use super::*;

#[test]
fn test_bhvec_random() {
    let v1 = BHVec10240::random();
    let v2 = BHVec10240::random();
    assert_ne!(v1, v2);
}

#[test]
fn test_bhvec_xor_hamming() {
    let v1 = BHVec10240::random();
    let v2 = BHVec10240::random();
    let bound = v1.xor(&v2);
    let dist = v1.hamming(&v2);
    assert_eq!(bound.bits.iter().map(|w| w.count_ones()).sum::<u32>(), dist);
}

#[test]
fn test_bhvec_hamming_edge_cases() {
    let zero = BHVec10240::zero();
    let ones = BHVec10240 { bits: [!0u64; 160] };
    let mut edge_bits = [0u64; 160];
    edge_bits[0] = 1;
    edge_bits[159] = 1 << 63;
    let edges = BHVec10240 { bits: edge_bits };

    assert_eq!(zero.hamming(&zero), 0);
    assert_eq!(zero.hamming(&ones), 10240);
    assert_eq!(zero.hamming(&edges), 2);
}

#[test]
fn test_bhvec_hamming_matches_scalar_oracle() {
    let lhs = BHVec10240::new_seeded(42);
    let rhs = BHVec10240::new_seeded(84);
    let expected = lhs
        .bits
        .iter()
        .zip(&rhs.bits)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>();
    let actual = lhs.hamming(&rhs);

    assert_eq!(actual, expected);
}

#[test]
fn test_bhvec_hamming_visits_every_packed_word() {
    let zero = BHVec10240::zero();

    for word_idx in 0..BHVec10240::WORDS {
        let mut bits = [0u64; BHVec10240::WORDS];
        bits[word_idx] = 1 << (word_idx % 64);
        let single_bit = BHVec10240 { bits };

        assert_eq!(
            zero.hamming(&single_bit),
            1,
            "packed word {word_idx} was skipped"
        );
    }
}

#[test]
fn test_bhvec_hamming_scalar_fallback_matches_oracle() {
    // Force the scalar fallback directly: CI machines almost always take the
    // AVX2/NEON kernels, so this lane-skip-hazardous loop needs an explicit
    // test instead of relying on a non-AVX2 runner.
    let lhs = BHVec10240::new_seeded(7);
    let rhs = BHVec10240::new_seeded(99);
    let expected = lhs
        .bits
        .iter()
        .zip(&rhs.bits)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum::<u32>();
    let actual = crate::hyperdim_simd::hamming_distance_u64_scalar(&lhs.bits, &rhs.bits);
    assert_eq!(actual, expected);
}

#[test]
fn test_bhvec_permute() {
    let v1 = BHVec10240::random();
    let v2 = v1.permute(1);
    assert_ne!(v1, v2);
    let v3 = v2.permute(BHVec10240::DIMENSION - 1);
    assert_eq!(v1, v3);
}

#[test]
fn test_bhvec_roundtrip_hvec() {
    let h1 = HVec10240::random();
    let bh1 = BHVec10240::from_hvec(&h1);
    let h2 = bh1.to_hvec();
    assert_eq!(h1, h2);
}

#[test]
fn test_bhvec_to_bytes_and_from_bytes_roundtrip() {
    let bh1 = BHVec10240::random();
    let bytes = bh1.to_bytes();
    assert_eq!(bytes.len(), 1280);
    let bh2 = BHVec10240::from_bytes(&bytes).unwrap();
    assert_eq!(bh1, bh2);
}

/// Naive per-bit majority oracle (`count >= N/2 + 1`).
fn naive_bundle_majority(vectors: &[&BHVec10240]) -> BHVec10240 {
    let n = vectors.len();
    if n == 0 {
        return BHVec10240::zero();
    }
    if n == 1 {
        return *vectors[0];
    }
    let threshold = n / 2 + 1;
    let mut bits = [0u64; 160];
    for word_idx in 0..160 {
        for bit_idx in 0..64 {
            let mask = 1u64 << bit_idx;
            let count = vectors
                .iter()
                .filter(|v| (v.bits[word_idx] & mask) != 0)
                .count();
            if count >= threshold {
                bits[word_idx] |= mask;
            }
        }
    }
    BHVec10240 { bits }
}

#[test]
fn test_bhvec_bundle_empty_and_single() {
    assert_eq!(BHVec10240::bundle(&[]), BHVec10240::zero());
    let v = BHVec10240::new_seeded(42);
    assert_eq!(BHVec10240::bundle(&[&v]), v);
}

#[test]
fn test_bhvec_bundle_n2_is_and() {
    let v1 = BHVec10240::new_seeded(1);
    let v2 = BHVec10240::new_seeded(2);
    let bundled = BHVec10240::bundle(&[&v1, &v2]);
    for i in 0..160 {
        assert_eq!(
            bundled.bits[i],
            v1.bits[i] & v2.bits[i],
            "N=2 must be bitwise AND at word {i}"
        );
    }
}

#[test]
fn test_bhvec_bundle_threshold_consistency() {
    // Span early returns, even-N ties, plane widths, and larger N (parity with HVec).
    for n in [2usize, 3, 4, 10, 255, 256, 1000] {
        let vectors: Vec<BHVec10240> =
            (0..n).map(|i| BHVec10240::new_seeded(i as u64)).collect();
        let refs: Vec<&BHVec10240> = vectors.iter().collect();
        let actual = BHVec10240::bundle(&refs);
        let expected = naive_bundle_majority(&refs);
        assert_eq!(
            actual.bits, expected.bits,
            "Bundling inconsistency at N={n} vectors"
        );
    }
}
