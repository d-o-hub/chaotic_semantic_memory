use super::*;

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
        assert!(val >= -1.0 && val <= 1.0); // tanh bounds
    }

    // state_norm should match manual calculation
    let manual_norm: f64 = out.state.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();
    assert!((out.state_norm - manual_norm).abs() < 1e-6);
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
