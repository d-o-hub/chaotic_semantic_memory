use chaotic_semantic_memory::reservoir::{ChaoticReservoir, Reservoir};

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

#[test]
fn test_reservoir_creation() {
    let r = Reservoir::new_seeded(10, 256, 42).unwrap();
    assert_eq!(r.size(), 256);
}

#[test]
fn test_reservoir_step() {
    let mut r = Reservoir::new_seeded(10, 256, 42).unwrap();
    assert_eq!(r.step(&[0.5; 10]).unwrap().state.len(), 256);
}

#[test]
fn test_spectral_radius_constraint() {
    let mut r = Reservoir::new_seeded(10, 256, 42).unwrap();
    assert!(r.set_spectral_radius(1.05).is_ok());
    assert!(r.set_spectral_radius(1.2).is_err());
}

#[test]
fn test_chaotic_reservoir() {
    let mut r = ChaoticReservoir::new_seeded(10, 256, 0.1, 42).unwrap();
    assert_eq!(r.step(&[0.5; 10]).unwrap().state.len(), 256);
}

#[test]
fn test_to_hypervector_small_reservoir_errors() {
    let r = Reservoir::new_seeded(10, 256, 42).unwrap();
    assert!(r.to_hypervector().is_err());
}
