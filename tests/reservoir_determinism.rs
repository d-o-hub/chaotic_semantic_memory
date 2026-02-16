use chaotic_semantic_memory::reservoir::ChaoticReservoir;

#[test]
fn reservoir_reset_and_seed_are_deterministic() {
    let mut r1 = ChaoticReservoir::new_seeded(8, 2048, 0.05, 7).unwrap();
    let mut r2 = ChaoticReservoir::new_seeded(8, 2048, 0.05, 7).unwrap();

    let input = vec![0.25; 8];

    r1.step(&input).unwrap();
    r2.step(&input).unwrap();
    assert_eq!(r1.state(), r2.state());

    r1.reset();
    r2.reset();
    assert_eq!(r1.state(), r2.state());

    r1.step(&input).unwrap();
    r2.step(&input).unwrap();
    assert_eq!(r1.state(), r2.state());
}
