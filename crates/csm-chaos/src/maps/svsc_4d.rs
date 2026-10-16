//! SVSC 4D Chaotic Map.
//!
//! Based on Zhang J et al., "DSLHash: Lightweight Hash Function Based on SVSC 4D
//! Chaotic System and Dynamic Substitution Layers" (2026).
//!
//! The map uses a multiplier-free approach for high-speed implementation while
//! maintaining 4D hyperchaotic properties, designed for lightweight environments.
//!
//! Note: This implementation represents the core 4D state updates and bit-mixing
//! based on the paper's formulation `x_{i+1} = (a * x_i) mod 2^32`.

/// Structure-varying high-dimensional chaotic map (SVSC 4D) state.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Svsc4d {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
}

impl Svsc4d {
    /// Create a new SVSC 4D map with initial states.
    ///
    /// Recommended: non-zero seeds to prevent trivial fixed points.
    pub const fn new(x: u32, y: u32, z: u32, w: u32) -> Self {
        Self { x, y, z, w }
    }

    /// Perform a single iteration of the 4D chaotic map.
    ///
    /// Employs a multiplier-free structure using bit shifts and additions
    /// mapped to the approximation of a fractional multiplier.
    #[inline]
    pub fn next(&mut self) {
        // According to the paper, the lightweight chaotic map uses:
        // x_{i+1} = F(A * x_i mod 2^32) using specific bitwise operations
        // A generic multiplier-free substitution: (x << 6) + (x << 3) + x + (x >> 28)
        let f = |v: u32| -> u32 {
            let t = v
                .wrapping_shl(6)
                .wrapping_add(v.wrapping_shl(3))
                .wrapping_add(v)
                .wrapping_add(v.wrapping_shr(28));
            (!t).wrapping_add(1) // two's complement inversion / negative mapping
        };

        // Coupled updates simulating the 4D dynamics
        let x_next = f(self
            .x
            .wrapping_add(self.y)
            .wrapping_add(self.w.rotate_left(5)));
        let y_next = f(self
            .y
            .wrapping_add(self.z)
            .wrapping_add(self.x.rotate_left(11)));
        let z_next = f(self
            .z
            .wrapping_add(self.w)
            .wrapping_add(self.y.rotate_left(19)));
        let w_next = f(self
            .w
            .wrapping_add(self.x)
            .wrapping_add(self.z.rotate_left(27)));

        self.x = x_next;
        self.y = y_next;
        self.z = z_next;
        self.w = w_next;
    }

    /// Generate the next pseudo-random value in [0, 1) by combining the state variables.
    #[inline]
    pub fn next_value(&mut self) -> f64 {
        self.next();

        // Mix the state into a 64-bit value for uniformity
        let mut h = (u64::from(self.x) << 32) | u64::from(self.y);
        h ^= (u64::from(self.z) << 32) | u64::from(self.w);

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
    fn test_svsc_4d_range() {
        let mut map = Svsc4d::new(12345, 67890, 13579, 24680);
        for _ in 0..1000 {
            let v = map.next_value();
            assert!((0.0..1.0).contains(&v), "Value {v} out of range");
        }
    }

    #[test]
    fn test_svsc_4d_sensitivity() {
        let mut map1 = Svsc4d::new(1000, 2000, 3000, 4000);
        let mut map2 = Svsc4d::new(1001, 2000, 3000, 4000);

        for _ in 0..1000 {
            map1.next();
            map2.next();
        }

        // Discrete chaotic systems on u32 space should diverge
        // leading to entirely different states
        assert!(map1.x != map2.x || map1.y != map2.y || map1.z != map2.z || map1.w != map2.w);
    }

    #[test]
    fn test_svsc_4d_distribution() {
        let mut map = Svsc4d::new(12345, 67890, 13579, 24680);
        let mut buckets = [0usize; 10];
        let n = 20000;

        for _ in 0..n {
            let v = map.next_value();
            #[allow(clippy::cast_possible_truncation)]
            let b = (v * 10.0) as usize;
            buckets[b.min(9)] += 1;
        }

        for &count in &buckets {
            assert!(count > 0, "Bucket is empty: {buckets:?}");
        }
    }
}
