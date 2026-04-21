//! Tests for InertialESN second-order momentum dynamics (ADR-0064).

use chaotic_semantic_memory::reservoir::{ChaoticReservoir, Reservoir};

#[test]
fn test_with_beta_valid_range() {
    // Test beta at boundaries and middle of valid range [0.0, 0.5]
    let r0 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.0);
    assert!(r0.is_ok());
    assert_eq!(r0.unwrap().beta(), 0.0);

    let r_mid = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.25);
    assert!(r_mid.is_ok());
    assert_eq!(r_mid.unwrap().beta(), 0.25);

    let r_max = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.5);
    assert!(r_max.is_ok());
    assert_eq!(r_max.unwrap().beta(), 0.5);
}

#[test]
fn test_with_beta_rejects_negative() {
    let result = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(-0.1);
    assert!(result.is_err());
}

#[test]
fn test_with_beta_rejects_above_max() {
    let result = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.6);
    assert!(result.is_err());

    let result2 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(1.0);
    assert!(result2.is_err());
}

#[test]
fn test_beta_getter_returns_correct_value() {
    let r = Reservoir::new_seeded(10, 256, 42)
        .unwrap()
        .with_beta(0.3)
        .unwrap();
    assert_eq!(r.beta(), 0.3);
}

#[test]
fn test_default_beta_is_zero() {
    // Default reservoir should have beta=0.0 (backward compatible)
    let r = Reservoir::new_seeded(10, 256, 42).unwrap();
    assert_eq!(r.beta(), 0.0);
}

#[test]
fn test_beta_zero_matches_original_behavior() {
    // Backward compatibility: beta=0.0 should produce identical state evolution
    let mut r1 = Reservoir::new_seeded(10, 256, 42).unwrap();
    let mut r2 = Reservoir::new_seeded(10, 256, 42)
        .unwrap()
        .with_beta(0.0)
        .unwrap();

    let input = vec![0.5; 10];

    // Multiple steps should produce identical state
    for _ in 0..5 {
        r1.step(&input).unwrap();
        r2.step(&input).unwrap();
        assert_eq!(r1.state(), r2.state(), "States diverged with beta=0.0");
    }
}

#[test]
fn test_chaotic_reservoir_step_with_inertia() {
    // ChaoticReservoir inherits inertial behavior from base Reservoir
    let mut r = ChaoticReservoir::new_seeded(10, 256, 0.1, 42).unwrap();

    // Should still function normally (default beta=0.0)
    let input = vec![0.5; 10];
    r.step(&input).unwrap();
    assert_eq!(r.state().len(), 256);
}

#[test]
fn test_beta_builder_is_chainable() {
    // Verify fluent builder pattern works with beta
    let r = Reservoir::new_seeded(10, 256, 42)
        .unwrap()
        .with_beta(0.1)
        .unwrap();

    assert_eq!(r.beta(), 0.1);
}
