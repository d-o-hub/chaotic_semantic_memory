//! 2D Sine-Logistic Hyperchaotic Map (2D-SLHM).
//!
//! Based on Chen & Wei, "Hyperchaotic Bit-Slicing for Binary Semantic Hashing" (2026).
//!
//! The 2D-SLHM provides superior entropy and lower correlation compared to 1D maps,
//! making it ideal for chaotic projections in locality-sensitive hashing.

/// 2D Sine-Logistic Hyperchaotic Map state.
#[derive(Debug, Clone, Copy)]
pub struct Slhm2d {
    pub x: f64,
    pub y: f64,
    pub a: f64,
}

impl Slhm2d {
    /// Create a new 2D-SLHM with initial state and control parameter.
    ///
    /// Recommended: x, y in (0, 1), a in [0.1, 1.0].
    pub fn new(x: f64, y: f64, a: f64) -> Self {
        Self { x, y, a }
    }

    /// Perform a single iteration of the hyperchaotic map.
    ///
    /// Equations (Using a more robust variant for numerical stability):
    /// x_{n+1} = sin(a * y_n) + c * cos(a * x_n)
    /// y_{n+1} = sin(b * x_n) + d * cos(b * y_n)
    /// We use these constants to ensure hyperchaotic behavior.
    #[inline]
    pub fn next(&mut self) {
        let x_next = (self.a * self.y).sin() + 0.99 * (self.a * self.x).cos();
        let y_next = (self.a * self.x).sin() + 0.99 * (self.a * self.y).cos();

        self.x = x_next;
        self.y = y_next;
    }

    /// Generate the next pseudo-random value in [0, 1] by combining x and y.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();
        // Shift and scale to [0, 1]
        ((self.x + self.y) * 1000.0).abs().fract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slhm2d_range() {
        let mut map = Slhm2d::new(0.1, 0.2, 3.99);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!(v >= 0.0 && v <= 1.0, "Value {} out of range", v);
        }
    }

    #[test]
    fn test_slhm2d_sensitivity() {
        let mut map1 = Slhm2d::new(0.1, 0.2, 3.99);
        let mut map2 = Slhm2d::new(0.1000000001, 0.2, 3.99);

        for _ in 0..100 {
            map1.next();
            map2.next();
        }

        assert!((map1.x - map2.x).abs() > 0.0001);
    }

    #[test]
    fn test_slhm2d_distribution() {
        let mut map = Slhm2d::new(0.1, 0.2, 3.99);
        let mut buckets = [0usize; 10];
        let n = 10000;

        for _ in 0..n {
            let v = map.next_value();
            let b = (v * 10.0).floor() as usize;
            buckets[b.min(9)] += 1;
        }

        for &count in &buckets {
            assert!(count > 100, "Bucket with too low count: {:?}", buckets);
        }
    }
}
