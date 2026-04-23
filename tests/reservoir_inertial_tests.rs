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

#[test]
fn test_beta_positive_changes_dynamics() {
    let mut r1 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.0).unwrap();
    let mut r2 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.15).unwrap();

    let mut input = vec![0.5; 10];
    for i in 0..100 {
        for j in 0..10 {
            input[j] = ((i + j) % 10) as f32 * 0.1;
        }
        r1.step(&input).unwrap();
        r2.step(&input).unwrap();
    }

    assert_ne!(r1.state(), r2.state(), "State with beta=0.15 should differ from beta=0.0");
}

#[test]
fn test_inertial_memory_length() {
    let mut r1 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.0).unwrap();
    let mut r2 = Reservoir::new_seeded(10, 256, 42).unwrap().with_beta(0.15).unwrap();

    let mut signal = vec![1.0; 10];
    let noise = vec![0.0; 10];

    for i in 0..100 {
        for j in 0..10 {
            signal[j] = ((i + j) % 10) as f32 * 0.1;
        }
        r1.step(&signal).unwrap();
        r2.step(&signal).unwrap();
    }

    let r1_state_0 = r1.state().to_vec();
    let r2_state_0 = r2.state().to_vec();

    for _ in 0..30 {
        r1.step(&noise).unwrap();
        r2.step(&noise).unwrap();
    }

    let cosine_sim = |a: &[f32], b: &[f32]| -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    };

    let sim1 = cosine_sim(&r1_state_0, r1.state());
    let sim2 = cosine_sim(&r2_state_0, r2.state());

    assert!(sim2 > sim1, "beta=0.15 should retain memory longer than beta=0.0 (sim2: {}, sim1: {})", sim2, sim1);
}
