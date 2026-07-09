//! 2D Sine-Logistic Hyperchaotic Map (2D-SLHM).
//!
//! Based on Chen & Wei, "Hyperchaotic Bit-Slicing for Binary Semantic Hashing" (2026).
//!
//! The 2D-SLHM provides superior entropy and lower correlation compared to 1D maps,
//! making it ideal for chaotic projections in locality-sensitive hashing.

/// 2D Sine-Logistic Hyperchaotic Map state.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Slhm2d {
    pub x: f64,
    pub y: f64,
    pub a: f64,
}

impl Slhm2d {
    /// Create a new 2D-SLHM with initial state and control parameter.
    ///
    /// Recommended: x, y in (0, 1), a in [0.9, 1.0] for maximum chaos.
    pub const fn new(x: f64, y: f64, a: f64) -> Self {
        Self { x, y, a }
    }

    /// Perform a single iteration of the hyperchaotic map.
    ///
    /// Equations:
    /// x_{n+1} = sin(π * (a * (y_n + 3) * x_n * (1 - x_n)))
    /// y_{n+1} = sin(π * (a * (x_{n+1} + 3) * y_n * (1 - y_n)))
    #[inline]
    pub fn next(&mut self) {
        let pi = core::f64::consts::PI;

        let x_next = libm::sin(pi * (self.a * (self.y + 3.0) * self.x * (1.0 - self.x)));
        let y_next = libm::sin(pi * (self.a * (x_next + 3.0) * self.y * (1.0 - self.y)));

        self.x = x_next;
        self.y = y_next;
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
    fn test_slhm2d_range() {
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!(v >= 0.0 && v < 1.0, "Value {} out of range", v);
        }
    }

    #[test]
    fn test_slhm2d_sensitivity() {
        let mut map1 = Slhm2d::new(0.1, 0.2, 0.99);
        let mut map2 = Slhm2d::new(0.1000000001, 0.2, 0.99);

        // Chaotic systems should diverge significantly within a few hundred iterations
        for _ in 0..200 {
            map1.next();
            map2.next();
        }

        assert!(libm::fabs(map1.x - map2.x) > 0.01);
    }

    #[test]
    fn test_slhm2d_distribution() {
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        let mut buckets = [0usize; 10];
        let n = 20000;

        for _ in 0..n {
            let v = map.next_value();
            let b = libm::floor(v * 10.0) as usize;
            buckets[b.min(9)] += 1;
        }

        // Check for representation in all buckets
        for &count in &buckets {
            assert!(count > 0, "Bucket is empty: {:?}", buckets);
        }
    }
}
