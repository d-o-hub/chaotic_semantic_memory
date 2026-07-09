//! 2D Chebyshev-Logistic Hyperchaotic Map.
//!
//! Provides a coupled chaotic system using Chebyshev polynomials of the first kind
//! and the Logistic map for high-entropy bit generation.

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChebyshevLogistic2d {
    pub x: f64,
    pub y: f64,
    pub a: f64, // Logistic control parameter, typically ~4.0
    pub k: i32, // Chebyshev degree, typically >= 2
}

impl ChebyshevLogistic2d {
    /// Create a new 2D Chebyshev-Logistic map.
    /// Recommended: x, y in (-1, 1), a in [3.57, 4.0], k >= 2.
    pub fn new(x: f64, y: f64, a: f64, k: i32) -> Self {
        debug_assert!(
            a > 3.57,
            "Logistic control parameter 'a' should be > 3.57 for chaotic behavior"
        );
        debug_assert!(
            k >= 2,
            "Chebyshev degree 'k' should be >= 2 (k=0, 1 are degenerate)"
        );
        Self { x, y, a, k }
    }

    /// Perform a single iteration of the coupled hyperchaotic map.
    #[inline]
    pub fn next(&mut self) {
        let pi = core::f64::consts::PI;

        // Chebyshev polynomial of the first kind: T_k(x) = cos(k * arccos(x))
        let chebyshev_x = libm::cos(self.k as f64 * libm::acos(self.x));

        // Logistic update: Map y from (-1, 1) to (0, 1) for logistic input, then back to (-1, 1)
        let y_norm = (self.y + 1.0) / 2.0;
        let logistic_y = self.a * y_norm * (1.0 - y_norm);
        let logistic_mapped = (logistic_y * 2.0) - 1.0;

        // Coupled hyperchaotic trajectory
        // trajectory enhancing coupling + sin(pi * (...)) for boundedness and sensitivity.
        // Multiplicative coupling with periodic perturbation to avoid fixed point at 0.
        let x_next = libm::sin(pi * (chebyshev_x * (1.0 - libm::fabs(self.y)) + 0.1));
        let y_next = libm::sin(pi * (logistic_mapped * (1.0 - libm::fabs(x_next)) + 0.1));

        // Clamp to valid range (-1.0, 1.0) to prevent domain errors in acos
        self.x = x_next.clamp(-0.999999, 0.999999);
        self.y = y_next.clamp(-0.999999, 0.999999);
    }

    /// Generate the next pseudo-random value in [0, 1) by combining x and y.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state
        let mut h = self.x.to_bits() ^ self.y.to_bits().rotate_left(32);

        // SplitMix64 finalizer for statistical uniformity
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
    fn test_chebyshev_logistic_range() {
        let mut map = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!(v >= 0.0 && v < 1.0, "Value {} out of range", v);
        }
    }

    #[test]
    fn test_chebyshev_logistic_determinism() {
        let mut map1 = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        let mut map2 = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);

        for _ in 0..100 {
            assert_eq!(map1.next_value(), map2.next_value());
        }
    }

    #[test]
    fn test_chebyshev_logistic_sensitivity() {
        let mut map1 = ChebyshevLogistic2d::new(0.1, 0.2, 4.0, 4);
        let mut map2 = ChebyshevLogistic2d::new(0.1000000001, 0.2, 4.0, 4);

        for _ in 0..1000 {
            map1.next();
            map2.next();
        }

        assert!(libm::fabs(map1.x - map2.x) > 0.01 || libm::fabs(map1.y - map2.y) > 0.01);
    }

    #[test]
    fn test_chebyshev_logistic_non_degenerate() {
        // Test that the map doesn't collapse to a constant or zero
        let mut map = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        let mut prev_x = map.x;
        let mut prev_y = map.y;
        let mut same_count = 0;

        for _ in 0..10000 {
            map.next();
            if libm::fabs(map.x - prev_x) < 1e-12 && libm::fabs(map.y - prev_y) < 1e-12 {
                same_count += 1;
            }
            prev_x = map.x;
            prev_y = map.y;

            assert!(libm::fabs(map.x) > 1e-10 || libm::fabs(map.y) > 1e-10);
        }

        assert!(
            same_count < 10,
            "Map seems to have converged: {}",
            same_count
        );
    }

    #[test]
    fn test_chebyshev_logistic_distribution() {
        let mut map = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        let mut buckets = [0usize; 10];
        let n = 20000;

        for _ in 0..n {
            let v = map.next_value();
            let b = libm::floor(v * 10.0) as usize;
            buckets[b.min(9)] += 1;
        }

        for &count in &buckets {
            assert!(count > 0, "Bucket is empty: {:?}", buckets);
        }
    }
}
