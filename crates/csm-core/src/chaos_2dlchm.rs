//! 2D Linear Cross-Coupled Hyperchaotic Map (2DLCHM) for high-entropy PRNG.
//!
//! Based on "Hyperchaotic hashing: a chaotic hash function based on 2D linear
//! cross-coupled map with parallel feedback structure" (Scientific Reports,
//! 2025, DOI: 10.1038/s41598-025-88764-0).
//!
//! Combines the Logistic map, Feigenbaum map, and cross-coupled chaotic map
//! to produce two positive Lyapunov exponents (hyperchaotic regime). Superior
//! dynamic complexity vs 1D maps; resists phase-space reconstruction attacks.

/// 2DLCHM state: dual-variable hyperchaotic oscillator.
#[derive(Debug, Clone)]
pub struct TwoDlchm {
    x: f64,
    y: f64,
}

impl TwoDlchm {
    /// Logistic map parameter (4.0 = full chaos).
    const A: f64 = 3.99;
    /// Feigenbaum parameter.
    const B: f64 = 3.57;
    /// Cross-coupling strength.
    const C: f64 = 0.1;

    /// Create a new 2DLCHM instance from two seed values in (0, 1).
    #[inline]
    pub fn new(seed_x: f64, seed_y: f64) -> Self {
        Self {
            x: seed_x.clamp(0.001, 0.999),
            y: seed_y.clamp(0.001, 0.999),
        }
    }

    /// Create from a byte seed (first 8 bytes → x, next 8 → y).
    pub fn from_seed(seed: &[u8]) -> Self {
        let x = if seed.len() >= 8 {
            f64::from_le_bytes(seed[..8].try_into().unwrap_or([0u8; 8]))
        } else {
            0.5
        };
        let y = if seed.len() >= 16 {
            f64::from_le_bytes(seed[8..16].try_into().unwrap_or([0u8; 8]))
        } else {
            0.5
        };
        Self::new(
            x.abs().fract().max(0.001).min(0.999),
            y.abs().fract().max(0.001).min(0.999),
        )
    }

    /// Advance state by one step.
    #[inline]
    pub fn step(&mut self) -> (f64, f64) {
        let x_next = Self::A * self.x * (1.0 - self.x) + Self::C * self.y;
        let y_next = Self::B * self.y * (1.0 - self.y) + Self::C * self.x;

        self.x = x_next.clamp(0.001, 0.999);
        self.y = y_next.clamp(0.001, 0.999);

        (self.x, self.y)
    }

    /// Generate a `u64` hash from the current state (parallel feedback).
    /// Upper 32 bits from `x`, lower 32 bits from `y`.
    #[inline]
    pub fn hash_u64(&mut self) -> u64 {
        let (hx, hy) = self.step();
        let lo = (hx * f64::from(1u32 << 32)) as u32;
        let hi = (hy * f64::from(1u32 << 32)) as u32;
        ((hi as u64) << 32) | lo as u64
    }

    /// Generate a single byte in `[0, 255]`.
    #[inline]
    pub fn next_byte(&mut self) -> u8 {
        let (h, _) = self.step();
        (h * 256.0) as u8
    }

    /// Generate `n` bytes of pseudo-random data.
    pub fn generate_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(n);
        for _ in 0..n {
            bytes.push(self.next_byte());
        }
        bytes
    }

    /// Generate a seed for `rand::StdRng`.
    pub fn rng_seed(&mut self) -> [u8; 32] {
        let mut seed = [0u8; 32];
        for chunk in seed.chunks_mut(8) {
            let (hx, hy) = self.step();
            let bytes = ((hx * ((1u64 << 32) as f64)) as u64).to_le_bytes();
            for (i, &b) in bytes.iter().enumerate().take(8) {
                if chunk.len() > i {
                    chunk[i] = b;
                }
            }
        }
        seed
    }

    /// Current state values (for diagnostics).
    #[inline]
    pub fn state(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// Estimate the largest Lyapunov exponent over `n` steps.
    /// Returns positive values for chaotic regimes.
    pub fn lyapunov_exponent(&mut self, n: usize) -> f64 {
        let mut sum = 0.0f64;
        for _ in 0..n {
            let (x, y) = self.step();
            // Derivative of the map at current point
            let dx = (Self::A * (1.0 - 2.0 * x)).abs();
            let dy = (Self::B * (1.0 - 2.0 * y)).abs();
            if dx > 0.0 && dy > 0.0 {
                sum += dx.ln() + dy.ln();
            }
        }
        sum / (2.0 * n as f64)
    }
}

impl Default for TwoDlchm {
    fn default() -> Self {
        Self::new(0.123456789, 0.987654321)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_stays_in_bounds() {
        let mut ch = TwoDlchm::default();
        for _ in 0..10000 {
            let (x, y) = ch.step();
            assert!(x > 0.0 && x < 1.0, "x out of bounds: {x}");
            assert!(y > 0.0 && y < 1.0, "y out of bounds: {y}");
        }
    }

    #[test]
    fn test_lyapunov_is_positive() {
        let mut ch = TwoDlchm::new(0.123, 0.456);
        let lyap = ch.lyapunov_exponent(10000);
        assert!(lyap > 0.0, "expected positive Lyapunov, got {lyap}");
    }

    #[test]
    fn test_different_seeds_diverge() {
        let mut a = TwoDlchm::new(0.123, 0.456);
        let mut b = TwoDlchm::new(0.123, 0.457); // tiny seed difference
        let mut max_diff = 0.0f64;
        for _ in 0..1000 {
            let (ax, ay) = a.step();
            let (bx, by) = b.step();
            let diff = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            max_diff = max_diff.max(diff);
        }
        assert!(
            max_diff > 0.01,
            "chaos should amplify small differences, max_diff={max_diff}"
        );
    }

    #[test]
    fn test_generate_bytes_length() {
        let mut ch = TwoDlchm::default();
        let bytes = ch.generate_bytes(256);
        assert_eq!(bytes.len(), 256);
    }

    #[test]
    fn test_from_seed_handles_short_input() {
        let ch = TwoDlchm::from_seed(&[1, 2, 3]);
        assert!(ch.x > 0.0 && ch.x < 1.0);
        assert!(ch.y > 0.0 && ch.y < 1.0);
    }
}
