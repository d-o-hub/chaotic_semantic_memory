//! 6D Asymmetrically Coupled Lorenz Map.
//!
//! Based on Hamdi et al., "A Novel Hyperchaotic System Derived from Asymmetric
//! Bidirectional Coupling of Two Lorenz Oscillators" (2026).
//!
//! Evaluates the 6D coupled vector field using 4th-order Runge-Kutta (RK4) discretisation
//! and extracts entropy from the coupled state for generation of uniform random numbers.

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoupledLorenzMap {
    pub x1: f64,
    pub y1: f64,
    pub z1: f64,
    pub x2: f64,
    pub y2: f64,
    pub z2: f64,
    pub alpha1: f64,
    pub alpha2: f64,
    pub dt: f64,
}

impl CoupledLorenzMap {
    /// Create a new 6D Coupled Lorenz Map.
    ///
    /// Recommended: x1, y1, z1, x2, y2, z2 not all zero.
    /// Classical Lorenz parameters: sigma=10, rho=28, beta=8/3 are hardcoded.
    ///
    /// # Panics
    ///
    /// Panics if `dt` is not positive (including `NaN`), if `dt` is not finite, if either
    /// coupling coefficient is not finite, or if the whole initial state is zero (the
    /// origin is an equilibrium, so the map would stay there forever).
    pub fn new(
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
        alpha1: f64,
        alpha2: f64,
        dt: f64,
    ) -> Self {
        assert!(dt > 0.0, "Time step dt must be positive");
        assert!(dt.is_finite(), "Time step dt must be finite");
        assert!(
            alpha1.is_finite() && alpha2.is_finite(),
            "Coupling coefficients must be finite"
        );
        assert!(
            !(x1 == 0.0 && y1 == 0.0 && z1 == 0.0 && x2 == 0.0 && y2 == 0.0 && z2 == 0.0),
            "Initial state cannot be completely zero"
        );

        Self {
            x1,
            y1,
            z1,
            x2,
            y2,
            z2,
            alpha1,
            alpha2,
            dt,
        }
    }

    /// Evaluates the vector field derivatives.
    #[inline]
    fn derivatives(
        &self,
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let sigma = 10.0;
        let rho = 28.0;
        let beta = 8.0 / 3.0;

        let dx1 = sigma * (y1 - x1) + self.alpha1 * (x2 - x1);
        let dy1 = x1 * (rho - z1) - y1 + self.alpha1 * (y2 - y1);
        let dz1 = x1 * y1 - beta * z1 + self.alpha1 * (z2 - z1);

        let dx2 = sigma * (y2 - x2) + self.alpha2 * (x1 - x2);
        let dy2 = x2 * (rho - z2) - y2 + self.alpha2 * (y1 - y2);
        let dz2 = x2 * y2 - beta * z2 + self.alpha2 * (z1 - z2);

        (dx1, dy1, dz1, dx2, dy2, dz2)
    }

    /// Perform a single RK4 step iteration of the coupled hyperchaotic system.
    #[inline]
    pub fn next(&mut self) {
        let dt = self.dt;
        let half_dt = dt * 0.5;

        // k1
        let (k1_x1, k1_y1, k1_z1, k1_x2, k1_y2, k1_z2) =
            self.derivatives(self.x1, self.y1, self.z1, self.x2, self.y2, self.z2);

        // k2
        let (k2_x1, k2_y1, k2_z1, k2_x2, k2_y2, k2_z2) = self.derivatives(
            self.x1 + half_dt * k1_x1,
            self.y1 + half_dt * k1_y1,
            self.z1 + half_dt * k1_z1,
            self.x2 + half_dt * k1_x2,
            self.y2 + half_dt * k1_y2,
            self.z2 + half_dt * k1_z2,
        );

        // k3
        let (k3_x1, k3_y1, k3_z1, k3_x2, k3_y2, k3_z2) = self.derivatives(
            self.x1 + half_dt * k2_x1,
            self.y1 + half_dt * k2_y1,
            self.z1 + half_dt * k2_z1,
            self.x2 + half_dt * k2_x2,
            self.y2 + half_dt * k2_y2,
            self.z2 + half_dt * k2_z2,
        );

        // k4
        let (k4_x1, k4_y1, k4_z1, k4_x2, k4_y2, k4_z2) = self.derivatives(
            self.x1 + dt * k3_x1,
            self.y1 + dt * k3_y1,
            self.z1 + dt * k3_z1,
            self.x2 + dt * k3_x2,
            self.y2 + dt * k3_y2,
            self.z2 + dt * k3_z2,
        );

        let dt_6 = dt / 6.0;
        self.x1 += dt_6 * (k1_x1 + 2.0 * k2_x1 + 2.0 * k3_x1 + k4_x1);
        self.y1 += dt_6 * (k1_y1 + 2.0 * k2_y1 + 2.0 * k3_y1 + k4_y1);
        self.z1 += dt_6 * (k1_z1 + 2.0 * k2_z1 + 2.0 * k3_z1 + k4_z1);
        self.x2 += dt_6 * (k1_x2 + 2.0 * k2_x2 + 2.0 * k3_x2 + k4_x2);
        self.y2 += dt_6 * (k1_y2 + 2.0 * k2_y2 + 2.0 * k3_y2 + k4_y2);
        self.z2 += dt_6 * (k1_z2 + 2.0 * k2_z2 + 2.0 * k3_z2 + k4_z2);
    }

    /// Generate the next pseudo-random value in [0, 1).
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state to extract entropy and ensure uniformity.
        let mut h = self.x1.to_bits()
            ^ self.y1.to_bits().rotate_left(11)
            ^ self.z1.to_bits().rotate_left(22)
            ^ self.x2.to_bits().rotate_left(33)
            ^ self.y2.to_bits().rotate_left(44)
            ^ self.z2.to_bits().rotate_left(55);

        // SplitMix64 finalizer
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;

        // Uniform f64 in [0, 1)
        #[allow(clippy::cast_precision_loss)]
        let result = (h >> 11) as f64 / (1u64 << 53) as f64;
        result
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    #[should_panic(expected = "Time step dt must be positive")]
    fn test_coupled_lorenz_rejects_non_positive_dt() {
        let _ = CoupledLorenzMap::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.0);
    }

    #[test]
    #[should_panic(expected = "Initial state cannot be completely zero")]
    fn test_coupled_lorenz_rejects_zero_initial_state() {
        let _ = CoupledLorenzMap::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.5, 0.5, 0.01);
    }

    #[test]
    fn test_coupled_lorenz_range() {
        let mut map = CoupledLorenzMap::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.01);
        for _ in 0..20000 {
            let v = map.next_value();
            assert!((0.0..1.0).contains(&v), "Value {v} out of range");
        }
    }

    #[test]
    fn test_coupled_lorenz_sensitivity() {
        let mut map1 = CoupledLorenzMap::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.01);
        let mut map2 = CoupledLorenzMap::new(1.0000000001, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.01);

        for _ in 0..20000 {
            map1.next();
            map2.next();
        }

        assert!(libm::fabs(map1.x1 - map2.x1) > 0.01);
    }

    #[test]
    fn test_coupled_lorenz_distribution() {
        let mut map = CoupledLorenzMap::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.5, 0.5, 0.01);
        let mut buckets = [0usize; 10];
        let n = 20000;

        for _ in 0..n {
            let v = map.next_value();
            #[allow(clippy::cast_possible_truncation)]
            let b = libm::floor(v * 10.0) as usize;
            buckets[b.min(9)] += 1;
        }

        for &count in &buckets {
            assert!(count > 0, "Bucket is empty: {buckets:?}");
        }
    }
}
