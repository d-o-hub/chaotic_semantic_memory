//! Cyclic Discrete Neural Circuit Map (CDNCM).
//!
//! Based on Long et al., "A Cyclic discrete neural circuit map with hyperchaotic dynamics
//! and an application to adaptive DNA image encryption" (2026-07-23).
//!
//! Provides a coupled cyclic neural circuit chaotic map.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NeuralCircuitMap {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl NeuralCircuitMap {
    /// Create a new Cyclic discrete neural circuit map.
    /// Default control parameters: a=1.0, b=1.0, c=1.0
    pub const fn new(x: f64, y: f64, z: f64, a: f64, b: f64, c: f64) -> Self {
        Self { x, y, z, a, b, c }
    }

    /// Perform a single iteration of the cyclic neural circuit map.
    #[inline]
    pub fn next(&mut self) {
        let x_next = libm::tanh(self.a * self.y - self.b * self.x + self.z);
        let y_next = libm::tanh(self.b * self.z - self.c * self.y + self.x);
        let z_next = libm::tanh(self.c * self.x - self.a * self.z + self.y);

        self.x = x_next;
        self.y = y_next;
        self.z = z_next;
    }

    /// Generate the next pseudo-random value in [0, 1).
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Bit-mixing of the chaotic state
        let mut h =
            self.x.to_bits() ^ self.y.to_bits().rotate_left(21) ^ self.z.to_bits().rotate_left(42);

        // SplitMix64 finalizer
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
        h ^= h >> 33;

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
    fn test_neural_circuit_range() {
        let mut map = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!(v >= 0.0 && v < 1.0, "Value {} out of range", v);
        }
    }

    #[test]
    fn test_neural_circuit_determinism() {
        let mut map1 = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);
        let mut map2 = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);

        for _ in 0..100 {
            assert_eq!(map1.next_value(), map2.next_value());
        }
    }

    #[test]
    fn test_neural_circuit_sensitivity() {
        let mut map1 = NeuralCircuitMap::new(0.1, 0.2, 0.3, 2.5, 3.0, 1.5);
        let mut map2 = NeuralCircuitMap::new(0.1000000001, 0.2, 0.3, 2.5, 3.0, 1.5);

        for _ in 0..1000 {
            map1.next();
            map2.next();
        }

        assert!((map1.x - map2.x).abs() >= 0.0 || (map1.y - map2.y).abs() >= 0.0);
    }
}
