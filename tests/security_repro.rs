use chaotic_semantic_memory::reservoir::ChaoticReservoir;

#[test]
fn test_chaos_strength_validation() {
    let res = ChaoticReservoir::new(10, 100, f32::NAN);
    assert!(res.is_err());

    let res = ChaoticReservoir::new(10, 100, -1.0);
    assert!(res.is_err());
}

#[test]
fn test_chaos_strength_zero() {
    let mut reservoir = ChaoticReservoir::new(10, 100, 0.0).unwrap();
    let input = vec![0.0; 10];
    let res = reservoir.step(&input);
    assert!(res.is_ok());
}

#[test]
fn test_reservoir_size_validation() {
    let res = ChaoticReservoir::new(10, 0, 0.1);
    assert!(res.is_err());

    let res = ChaoticReservoir::new(10, 200_000, 0.1);
    assert!(res.is_err());
}
