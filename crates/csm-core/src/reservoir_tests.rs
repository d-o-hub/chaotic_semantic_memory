use super::*;
use crate::reservoir::Reservoir;
use crate::reservoir::Reservoir;

#[test]
fn new_valid() {
    let r = Reservoir::new(1024, 10240).unwrap();
    assert_eq!(r.size(), 10240);
}

#[test]
fn new_invalid_size() {
    assert!(Reservoir::new(1024, 0).is_err());
    assert!(Reservoir::new(1024, 200_000).is_err());
}

#[test]
fn step_ok() {
    let mut r = Reservoir::new(1024, 10240).unwrap();
    let out = r.step(&[0.0; 1024]).unwrap();
    assert_eq!(out.state.len(), 10240);
}

#[test]
fn reset_clears() {
    let mut r = Reservoir::new(1024, 10240).unwrap();
    let _ = r.step(&[1.0; 1024]);
    r.reset();
    assert!(r.state().iter().all(|x| *x == 0.0));
}

#[test]
fn spectral_radius_bounds() {
    let mut r = Reservoir::new(1024, 10240).unwrap();
    assert!(r.set_spectral_radius(0.8).is_err());
    r.set_spectral_radius(1.0).unwrap();
}

#[test]
fn metrics_steps() {
    let mut r = Reservoir::new(1024, 10240).unwrap();
    let _ = r.step(&[0.0; 1024]);
    assert_eq!(r.metrics_snapshot().reservoir_steps_total, 1);
}

#[test]
fn step_mathematical_correctness() {
    // Small reservoir to verify exact math
    let mut r = Reservoir::new_seeded(2, 4, 42).unwrap();
    r.alpha = 1.0; // Simplify: new_state = activated
    r.beta = 0.0; // No inertia
    r.update_stride = 1; // Full update every step

    let input = [0.5, -0.5];
    let out = r.step(&input).unwrap();

    // Verify each node calculation manually if possible, or just consistency
    assert_eq!(out.state.len(), 4);
    for &val in out.state {
        assert!((-1.0..=1.0).contains(&val)); // tanh bounds
    }

    // state_norm should match manual calculation
    let manual_norm: f64 = out
        .state
        .iter()
        .map(|&x| (x as f64) * (x as f64))
        .sum::<f64>()
        .sqrt();
    assert!((out.state_norm - manual_norm).abs() < 1e-6);
}

#[test]
#[allow(clippy::float_cmp)]
fn norm_calculation_consistency() {
    // 1. Verify direct multiplication matches powi(2) for range of values
    // Using to_bits() to bypass clippy::float_cmp while ensuring bit-identical results
    for i in -100..100 {
        let x = i as f64 * 0.01;
        assert_eq!((x * x).to_bits(), x.powi(2).to_bits());
    }

    // 2. Verify reservoir state_norm consistency over many steps
    // Use a small reservoir and update_stride for test speed
    let mut r = Reservoir::new_seeded(10, 100, 42).unwrap();
    // update_stride is 32 by default. Run 100 steps to trigger 3+ full re-calculations.
    let input = vec![0.1; 10];

    for i in 0..100 {
        let out = r.step(&input).unwrap();

        // Manual calculation of norm from the actual state
        let manual_norm_sq: f64 = out.state.iter().map(|&x| f64::from(x) * f64::from(x)).sum();
        let manual_norm = manual_norm_sq.sqrt();

        // Precision drift should be minimal (within 1e-9 for f64 accumulation)
        assert!(
            (out.state_norm - manual_norm).abs() < 1e-9,
            "Norm mismatch at step {}: internal={}, manual={}",
            i,
            out.state_norm,
            manual_norm
        );

        if r.update_phase == 0 {
            // This was a full re-calculation step (or it will be at the start of next step if we check before increment)
            // Actually in code: `if update_phase == 0 { re-calculate }` happens BEFORE `update_phase` is incremented.
            // But we already called step, so `r.update_phase` was just incremented.
            // If it's now 1, the previous step was phase 0.
        }
    }
}

#[test]
fn step_short_input_returns_error() {
    let mut r = Reservoir::new(1024, 10240).unwrap();
    // Providing shorter input than expected should trigger the size check
    let res = r.step(&[0.0; 512]);
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(err.contains("Input size mismatch"));
}
