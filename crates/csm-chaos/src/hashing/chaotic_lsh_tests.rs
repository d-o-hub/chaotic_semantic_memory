#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::hashing::MAX_INPUT_DIM;

#[test]
fn try_new_rejects_zero_and_oversized_dimensions() {
    assert!(ChaoticLsh::try_new(0.5, 0.7, 0.95, 0).is_err());
    assert!(ChaoticLsh::try_new(0.5, 0.7, 0.95, MAX_INPUT_DIM + 1).is_err());
    assert_eq!(
        ChaoticLsh::try_new(0.5, 0.7, 0.95, 8)
            .expect("an in-range dimension is accepted")
            .input_dim(),
        8
    );
}

#[test]
fn new_reports_the_dimension_it_actually_uses() {
    let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 0);
    assert_eq!(lsh.input_dim(), 1);
}

#[test]
fn shape_mismatch_returns_an_all_zero_hypervector() {
    let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 4);
    // The hazard the docs warn about: an input of the wrong length is not
    // projected, it silently yields the empty code.
    assert_eq!(lsh.project(&[1.0; 5]), [0u64; 160]);
    assert_eq!(lsh.project_scalar(&[1.0; 5]), [0u64; 160]);
    assert_eq!(lsh.project_simd(&[1.0; 5]), [0u64; 160]);
}

#[test]
fn test_chaotic_lsh_locality() {
    let input_dim = 4;
    let lsh = ChaoticLsh::new(0.123, 0.456, 0.99, input_dim);

    let v1 = [1.0, 0.5, -0.2, 0.8];
    let v2 = [1.0, 0.5, -0.2, 0.81];
    let v3 = [-1.0, -0.5, 0.2, -0.8];

    let h1 = lsh.project(&v1);
    let h2 = lsh.project(&v2);
    let h3 = lsh.project(&v3);

    fn hamming(a: &[u64; 160], b: &[u64; 160]) -> u32 {
        let mut dist = 0u32;
        for i in 0..160 {
            dist += (a[i] ^ b[i]).count_ones();
        }
        dist
    }

    fn cosine_similarity(a: &[u64; 160], b: &[u64; 160]) -> f32 {
        let dist = hamming(a, b);
        #[allow(clippy::cast_precision_loss)]
        (1.0 - (dist as f32 / 5120.0))
    }

    let sim_12 = cosine_similarity(&h1, &h2);
    let sim_13 = cosine_similarity(&h1, &h3);

    assert!(
        sim_12 > 0.8,
        "Similar vectors should have high similarity: {sim_12}"
    );
    assert!(
        sim_13 < 0.6,
        "Different vectors should have low similarity: {sim_13}"
    );
}

#[test]
fn test_simd_scalar_parity() {
    let lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 8);
    let input = [0.1, -0.3, 0.5, 0.7, -0.2, 0.4, -0.6, 0.8];
    let scalar = lsh.project_scalar(&input);
    let result = lsh.project(&input);
    assert_eq!(scalar, result);
}

#[test]
fn test_scalar_projection_includes_input_remainder() {
    let mut lsh = ChaoticLsh::new(0.5, 0.7, 0.95, 5);
    for row in lsh.projection_matrix.as_chunks_mut::<5>().0 {
        row.fill(0.0);
        row[4] = 1.0;
    }

    let input = [0.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(lsh.project_scalar(&input), [u64::MAX; 160]);
}
