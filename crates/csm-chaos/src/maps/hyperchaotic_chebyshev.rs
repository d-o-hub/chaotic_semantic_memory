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
    pub k: i32, // Chebyshev degree, typically >= 3
}

impl ChebyshevLogistic2d {
    /// Create a new 2D Chebyshev-Logistic map.
    /// Recommended: x, y in (-1, 1), a = 4.0, k = 4.
    pub fn new(x: f64, y: f64, a: f64, k: i32) -> Self {
        Self { x, y, a, k }
    }

    /// Perform a single iteration of the coupled hyperchaotic map.
    #[inline]
    pub fn next(&mut self) {
        // Chebyshev polynomial of the first kind: T_k(x) = cos(k * arccos(x))
        let chebyshev_x = libm::cos(self.k as f64 * libm::acos(self.x));

        // Coupled logistic update
        // Map y from (-1, 1) to (0, 1) for logistic input, then map back to (-1, 1)
        let y_norm = (self.y + 1.0) / 2.0;
        let logistic_y = self.a * y_norm * (1.0 - y_norm);
        let logistic_mapped = (logistic_y * 2.0) - 1.0;

        // Coupling
        let x_next = chebyshev_x * (1.0 - libm::fabs(self.y));
        let y_next = logistic_mapped * (1.0 - libm::fabs(self.x));

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
        (h >> 11) as f64 / (1u64 << 53) as f64
    }
}
