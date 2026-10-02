//! Complex Quadratic Map.
//!
//! Based on Ayubi et al., "Chaotic Complex Hashing: A simple chaotic keyed hash
//! function based on complex quadratic map" (2023). DOI: 10.1016/j.chaos.2023.113647.
//!
//! The map iterates the function $z_{n+1} = z_n^2 + c$ in the complex plane,
//! corresponding to the equations $x_{n+1} = x_n^2 - y_n^2 + c_x$ and
//! $y_{n+1} = 2 x_n y_n + c_y$.

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComplexQuadraticMap {
    pub x: f64,
    pub y: f64,
    pub cx: f64,
    pub cy: f64,
}

impl ComplexQuadraticMap {
    /// Create a new Complex Quadratic Map with initial state $(x, y)$ and parameters $(c_x, c_y)$.
    ///
    /// For chaotic behavior, $(c_x, c_y)$ should be chosen near the boundary of the
    /// Mandelbrot set.
    pub const fn new(x: f64, y: f64, cx: f64, cy: f64) -> Self {
        Self { x, y, cx, cy }
    }

    /// Perform a single iteration of the complex quadratic map.
    #[inline]
    pub fn next(&mut self) {
        let x_next = self.x * self.x - self.y * self.y + self.cx;
        let y_next = 2.0 * self.x * self.y + self.cy;

        // Prevent divergence beyond the threshold |z| > 2, mapped back dynamically
        if x_next * x_next + y_next * y_next > 4.0 {
            self.x = (x_next % 2.0) - 1.0;
            self.y = (y_next % 2.0) - 1.0;
        } else {
            self.x = x_next;
            self.y = y_next;
        }
    }

    /// Generate the next pseudo-random value in [0, 1) by combining x and y.
    ///
    /// Uses a combined bit-mixing approach for maximum statistical uniformity.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state to extract entropy and ensure uniformity.
        let mut h = self.x.to_bits() ^ self.y.to_bits().rotate_left(32);

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
    fn test_cqm_range() {
        let mut map = ComplexQuadraticMap::new(0.1, 0.2, -0.8, 0.156);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!((0.0..1.0).contains(&v), "Value {v} out of range");
        }
    }

    #[test]
    fn test_cqm_sensitivity() {
        let mut map1 = ComplexQuadraticMap::new(0.1, 0.2, -0.8, 0.156);
        let mut map2 = ComplexQuadraticMap::new(0.1000000001, 0.2, -0.8, 0.156);

        // Chaotic systems should diverge significantly, but might need more iterations
        // in some regimes.
        for _ in 0..1000 {
            map1.next();
            map2.next();
        }

        assert!(libm::fabs(map1.x - map2.x) > 0.01 || libm::fabs(map1.y - map2.y) > 0.01);
    }

    #[test]
    fn test_cqm_distribution() {
        let mut map = ComplexQuadraticMap::new(0.123, 0.456, -0.8, 0.156);
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
