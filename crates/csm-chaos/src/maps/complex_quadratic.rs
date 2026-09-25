//! Complex Quadratic Chaotic Map.
//!
//! Based on Ayubi et al., "Chaotic Complex Hashing: A simple chaotic keyed
//! hash function based on complex quadratic map" (2023).
//!
//! Provides a chaotic map based on complex arithmetic: z_{n+1} = z_n^2 + c.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Complex Quadratic Chaotic Map state.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComplexQuadraticMap {
    pub real_z: f64,
    pub imag_z: f64,
    pub real_c: f64,
    pub imag_c: f64,
}

impl ComplexQuadraticMap {
    /// Create a new Complex Quadratic Map with initial state z_0 and parameter c.
    /// Recommended: z and c should be chosen such that the sequence remains bounded and chaotic
    /// (e.g., within the Mandelbrot/Julia set boundaries for chaos).
    pub const fn new(real_z: f64, imag_z: f64, real_c: f64, imag_c: f64) -> Self {
        Self {
            real_z,
            imag_z,
            real_c,
            imag_c,
        }
    }

    /// Perform a single iteration of the complex quadratic map.
    ///
    /// Equation: z_{n+1} = z_n^2 + c
    /// Re(z_{n+1}) = Re(z_n)^2 - Im(z_n)^2 + Re(c)
    /// Im(z_{n+1}) = 2 * Re(z_n) * Im(z_n) + Im(c)
    #[inline]
    pub fn next(&mut self) {
        let r2 = self.real_z * self.real_z;
        let i2 = self.imag_z * self.imag_z;

        let next_r = r2 - i2 + self.real_c;
        let next_i = 2.0 * self.real_z * self.imag_z + self.imag_c;

        self.real_z = next_r;
        self.imag_z = next_i;
    }

    /// Generate the next pseudo-random value in [0, 1) by combining real and imaginary parts.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state to extract entropy and ensure uniformity.
        let mut h = self.real_z.to_bits() ^ self.imag_z.to_bits().rotate_left(32);

        // SplitMix64 finalizer
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;

        // Uniform f64 in [0, 1)
        #[allow(clippy::cast_precision_loss)] // Standard u64→f64 for uniform random generation
        let result = (h >> 11) as f64 / (1u64 << 53) as f64;
        result
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_complex_quadratic_range() {
        let mut map = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!((0.0..1.0).contains(&v), "Value {v} out of range");
        }
    }

    #[test]
    fn test_complex_quadratic_sensitivity() {
        let mut map1 = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
        let mut map2 = ComplexQuadraticMap::new(0.1000000001, 0.2, -0.75, 0.1);

        for _ in 0..35 {
            map1.next();
            map2.next();
        }

        assert!(
            libm::fabs(map1.real_z - map2.real_z) > 1e-5
                || libm::fabs(map1.imag_z - map2.imag_z) > 1e-5
        );
    }

    #[test]
    fn test_complex_quadratic_determinism() {
        let mut map1 = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
        let mut map2 = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);

        for _ in 0..100 {
            #[allow(clippy::float_cmp)]
            let v1 = map1.next_value();
            let v2 = map2.next_value();
            #[allow(clippy::float_cmp)]
            {
                assert_eq!(v1, v2);
            }
        }
    }

    #[test]
    fn test_complex_quadratic_distribution() {
        let mut map = ComplexQuadraticMap::new(0.123, 0.456, -0.75, 0.1);
        let mut buckets = [0usize; 10];
        let n = 20000;

        for _ in 0..n {
            let v = map.next_value();
            #[allow(clippy::cast_possible_truncation)]
            let b = libm::floor(v * 10.0) as usize;
            buckets[b.min(9)] += 1;
        }

        // Check for representation in all buckets
        for &count in &buckets {
            assert!(count > 0, "Bucket is empty: {buckets:?}");
        }
    }
}
