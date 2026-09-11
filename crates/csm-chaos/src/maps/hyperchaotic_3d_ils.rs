//! 3D-ILS chaotic map (3D-Cascading Crossing Coupling over ICMIC/Logistic/Sine).
//!
//! Inspired by Sun et al., "A 3D-Cascading Crossing Coupling Framework for Hyperchaotic
//! Map Construction and Its Application to Color Image Encryption" (2025).
//!
//! [`Ils3d::next`] cascades the three 1D maps with the paper's denominator guard
//! (`den()`) and bounded-state clamp (`sat()`). It is not a bit-exact port of the
//! paper's Eq. 9: the Logistic and Sine updates fold through `sin(pi * .)` before
//! clamping, and [`Ils3d::next_value`] mixes the coupled state through a SplitMix64
//! finalizer, so the emitted stream is not the paper's keystream. No Lyapunov or
//! bifurcation evidence is asserted here.
//!
//! The map is experimental and feature gated; it is not wired into the LSH hashing
//! paths, which use [`crate::maps::hyperchaotic::Slhm2d`].

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ils3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub a1: f64, // ICMIC parameter
    pub a2: f64, // Logistic parameter
    pub a3: f64, // Sine parameter
}

impl Ils3d {
    /// Create a new 3D-ILS map.
    /// Recommended: x, y, z in (0, 1). a1, a2, a3 are control parameters.
    /// Default values often used in chaos: a1=21.0, a2=4.0, a3=4.0
    pub const fn new(x: f64, y: f64, z: f64, a1: f64, a2: f64, a3: f64) -> Self {
        Self {
            x,
            y,
            z,
            a1,
            a2,
            a3,
        }
    }

    /// Perform a single iteration of the coupled hyperchaotic map.
    #[inline]
    pub fn next(&mut self) {
        let pi = core::f64::consts::PI;

        // Based on 3D-CCC principles for ICMIC, Logistic, Sine (ILS)
        // x_{n+1} = ICMIC like update crossed with Logistic/Sine
        // For standard ICMIC: x_{n+1} = sin(a1 / x_n)
        // With coupling:
        let epsilon = 1e-6; // Denominator safeguard
        let x_denom = if libm::fabs(self.x) < epsilon {
            epsilon
        } else {
            self.x
        };
        let x_next = libm::sin(self.a1 / x_denom + pi * (self.y + self.z));

        // Logistic update
        let y_next = self.a2 * self.y * (1.0 - self.y) + x_next * self.z;
        // Bounded state safeguard for y_next into (0, 1) using a modulo-1 equivalent or clamping
        // Sine mapping often used to bound:
        let y_next_bounded = libm::sin(pi * y_next);
        let y_final = libm::fabs(y_next_bounded);

        // Sine update
        let z_next = self.a3 * libm::sin(pi * self.z) + x_next * y_final;
        let z_next_bounded = libm::sin(pi * z_next);
        let z_final = libm::fabs(z_next_bounded);

        self.x = x_next;
        self.y = y_final.clamp(0.000001, 0.999999);
        self.z = z_final.clamp(0.000001, 0.999999);
    }

    /// Generate the next pseudo-random value in [0, 1) by combining x, y, and z.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state
        let mut h =
            self.x.to_bits() ^ self.y.to_bits().rotate_left(21) ^ self.z.to_bits().rotate_left(42);

        // SplitMix64 finalizer for statistical uniformity
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
    fn test_ils3d_range() {
        let mut map = Ils3d::new(0.123, 0.456, 0.789, 21.0, 4.0, 4.0);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!((0.0..1.0).contains(&v), "Value {v} out of range");
        }
    }

    #[test]
    fn test_ils3d_sensitivity() {
        let mut map1 = Ils3d::new(0.1, 0.2, 0.3, 21.0, 4.0, 4.0);
        let mut map2 = Ils3d::new(0.1000000001, 0.2, 0.3, 21.0, 4.0, 4.0);

        for _ in 0..1000 {
            map1.next();
            map2.next();
        }

        assert!(
            libm::fabs(map1.x - map2.x) > 0.01
                || libm::fabs(map1.y - map2.y) > 0.01
                || libm::fabs(map1.z - map2.z) > 0.01
        );
    }

    #[test]
    fn test_ils3d_distribution() {
        let mut map = Ils3d::new(0.123, 0.456, 0.789, 21.0, 4.0, 4.0);
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
