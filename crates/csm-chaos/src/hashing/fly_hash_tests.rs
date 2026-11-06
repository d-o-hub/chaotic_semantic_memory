use super::*;
use crate::hashing::MAX_INPUT_DIM;

#[test]
fn test_flyhash_initialization() {
    let input_dim = 128;
    let sparsity = 0.5_f32;
    let top_k = 500;

    let fh = FlyHash::new(0.1, 0.2, sparsity, top_k, input_dim);
    assert_eq!(fh.input_dim, input_dim);
    assert!((fh.sparsity - sparsity).abs() < f32::EPSILON);
    assert_eq!(fh.top_k, top_k);
    assert_eq!(fh.projection_matrix.len(), 10240 * input_dim);
}

#[test]
fn test_flyhash_winner_take_all() {
    let input_dim = 128;
    let top_k = 200;
    let fh = FlyHash::new(0.3, 0.4, 0.2, top_k, input_dim);

    #[allow(clippy::cast_precision_loss)]
    let input: alloc::vec::Vec<f32> = (0..input_dim).map(|i| (i as f32) / 100.0).collect();
    let code = fh.project(&input);

    let mut set_bits = 0;
    for word in code {
        set_bits += word.count_ones();
    }

    assert_eq!(
        set_bits as usize, top_k,
        "Winner-take-all did not set exactly top_k bits."
    );
}

#[test]
fn test_flyhash_determinism() {
    let input_dim = 64;
    let fh1 = FlyHash::new(0.5, 0.5, 0.15, 300, input_dim);
    let fh2 = FlyHash::new(0.5, 0.5, 0.15, 300, input_dim);

    #[allow(clippy::cast_precision_loss)]
    let input: alloc::vec::Vec<f32> = (0..input_dim).map(|i| (i as f32) / 50.0).collect();
    let code1 = fh1.project(&input);
    let code2 = fh2.project(&input);

    assert_eq!(
        code1, code2,
        "Outputs must be deterministic for identical seeds."
    );
}

#[test]
fn test_flyhash_dimension_bounds() {
    assert!(FlyHash::try_new(0.1, 0.2, 0.1, 100, 0).is_err());
    assert!(FlyHash::try_new(0.1, 0.2, 0.1, 100, MAX_INPUT_DIM + 1).is_err());
    assert!(FlyHash::try_new(0.1, 0.2, 0.1, 100, MAX_INPUT_DIM).is_ok());
}
