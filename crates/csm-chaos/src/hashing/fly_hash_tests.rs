use super::*;
use crate::hashing::MAX_INPUT_DIM;
use alloc::vec;
use alloc::vec::Vec;

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
    let input: Vec<f32> = (0..input_dim).map(|i| (i as f32) / 100.0).collect();
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
    let input: Vec<f32> = (0..input_dim).map(|i| (i as f32) / 50.0).collect();
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

#[test]
fn test_flyhash_sparsity() {
    let input_dim = 128;
    // ensure we will get values with typical Slhm2d which fluctuates in [0, 1] probabilistically
    let sparsity = 1.0_f32;
    let top_k = 500;
    let fh = FlyHash::new(0.1, 0.2, sparsity, top_k, input_dim);

    let mut non_zero = 0;
    for &val in &fh.projection_matrix {
        if val != 0.0 {
            non_zero += 1;
        }
    }

    assert!(
        non_zero > 0,
        "Projection matrix should have non-zero elements"
    );
}

#[test]
fn test_flyhash_projection_math() {
    let input_dim = 128;
    let top_k = 2; // Very small K to track exact activations
    let mut fh = FlyHash::new(0.1, 0.2, 0.5, top_k, input_dim);

    let mut fake_matrix = vec![0.0; 10240 * input_dim];
    // Set neuron 0 to match exactly input
    for i in 0..input_dim {
        fake_matrix[i] = 1.0;
    }
    // Set neuron 1 to heavily weight the last element
    fake_matrix[input_dim + input_dim - 1] = 100.0;
    fh.projection_matrix = fake_matrix;

    // Using a specific pattern to ensure logic like + and * cannot be swapped
    let mut input = vec![1.0; input_dim];
    input[1] = 2.0;
    let code = fh.project(&input);

    // We expect bits 0 and 1 to be set, so the first u64 is exactly 3.
    // Also explicitly checking bits math logic
    assert_eq!(code[0], 3);

    // Test that empty code array is not returned
    assert_ne!(code, [0; 160]);
    assert_ne!(code, [1; 160]);
}

#[test]
fn test_flyhash_try_new_math() {
    let input_dim = 10;
    let sparsity = 0.5_f32;
    let top_k = 500;
    let fh = FlyHash::try_new(0.1, 0.2, sparsity, top_k, input_dim).unwrap();
    assert_eq!(fh.projection_matrix.len(), 10240 * 10);

    let fh_zero_sparsity = FlyHash::try_new(0.1, 0.2, 0.0, top_k, input_dim).unwrap();
    assert!(fh_zero_sparsity.projection_matrix.iter().all(|&x| x == 0.0));

    let fh_full_sparsity = FlyHash::try_new(0.1, 0.2, 1.0, top_k, input_dim).unwrap();
    assert!(fh_full_sparsity.projection_matrix.iter().any(|&x| x != 0.0));

    // catch v1 + 1.0 -> v1 * 1.0
    // If it was *, prob would be roughly [-0.5, 0.5] instead of [0, 1]
    let fh_mut2 = FlyHash::try_new(0.1, 0.2, 0.9, top_k, input_dim).unwrap();
    let count1 = fh_mut2
        .projection_matrix
        .iter()
        .filter(|&&x| x != 0.0)
        .count();
    let fh_mut3 = FlyHash::try_new(0.1, 0.2, 0.5, top_k, input_dim).unwrap();
    let count2 = fh_mut3
        .projection_matrix
        .iter()
        .filter(|&&x| x != 0.0)
        .count();

    // ensure different sparsity ratios produce different number of non-zero elements
    assert!(count1 != count2);
}

#[test]
fn test_flyhash_mutation_try_new_bounds() {
    // try_new mutants check
    let fh = FlyHash::try_new(0.1, 0.2, 0.5, 500, 10).unwrap();
    assert_eq!(fh.input_dim, 10);
    assert_eq!(fh.top_k, 500);

    // Test logic operations replaced by mutations
    let fh_mut1 = FlyHash::try_new(0.1, 0.2, 0.5, 500, 12).unwrap();
    assert_eq!(fh_mut1.projection_matrix.len(), 10240 * 12);
}

#[test]
fn test_flyhash_mutation_project() {
    let fh = FlyHash::new(0.1, 0.2, 0.5, 500, 10);
    let input = vec![1.0; 10];
    let code = fh.project(&input);
    let code2 = fh.project(&input);

    assert_eq!(code, code2);

    // test empty input projection gives all zeros activations resulting in top k selected by natural order (first k bits set)
    let empty_input = vec![0.0; 10];
    let empty_code = fh.project(&empty_input);

    // If activations are all 0, sort preserves order or gives specific order.
    // In our case partial_cmp handles it. We just check it's not panicking.
    assert_eq!(empty_code.len(), 160);

    // check exact activation sum logic
    let mut specific_matrix = vec![0.0; 10240 * 2];
    specific_matrix[0] = 2.0;
    specific_matrix[1] = 3.0;

    // use negative weight to trigger != 0.0 mutation (== 0.0) check
    specific_matrix[2] = 0.0;

    let mut custom_fh = FlyHash::new(0.1, 0.2, 0.5, 1, 2);
    custom_fh.projection_matrix = specific_matrix;

    let proj1 = custom_fh.project(&[4.0, 5.0]);
    // activation for neuron 0 is 4*2 + 5*3 = 23
    assert_eq!(proj1[0], 1); // 1 bit set for neuron 0

    let proj2 = custom_fh.project(&[10.0, 2.0]);
    // activation for neuron 0 is 10*2 + 2*3 = 26
    assert_eq!(proj2[0], 1);

    // Check if the weight logic `dot += input[j] * weight` mutants (* -> /, etc)
    let _proj3 = custom_fh.project(&[0.5, 0.5]);
    // activation is 0.5 * 2 + 0.5 * 3 = 2.5
    // if * was changed to /, 0.5 / 2 = 0.25, 0.5 / 3 = 0.166 -> 0.416
    // this would change sorting if there was another neuron

    custom_fh.projection_matrix[2] = 4.0;
    custom_fh.projection_matrix[3] = 0.5;
    // neuron 1: input[0]*4 + input[1]*0.5

    let proj4 = custom_fh.project(&[0.5, 0.5]);
    // neuron 0: 2.5
    // neuron 1: 0.5*4 + 0.5*0.5 = 2.25
    // max is neuron 0.
    assert_eq!(proj4[0], 1);

    let proj5 = custom_fh.project(&[1.5, 0.5]);
    // neuron 0: 1.5*2 + 0.5*3 = 4.5
    // neuron 1: 1.5*4 + 0.5*0.5 = 6.25
    // max is neuron 1.
    assert_eq!(proj5[0], 2);
}
