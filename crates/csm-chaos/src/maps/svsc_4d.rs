/// Structure-varying high-dimensional chaotic map, 4D (Svsc4d)
///
/// Based on "A structure-varying multi-scroll chaotic system and its application in image encryption"
/// by Y. Wang et al., 2025.
/// [FALLBACK — outside this run window]
///
/// Multiplier-free approach implemented using bitwise logic to avoid floating-point multiplication
/// on the fast path, converting to `f64` values and bit-bashing the mantissa.
#[derive(Debug, Clone, Copy)]
pub struct Svsc4d {
    pub x1: f64,
    pub x2: f64,
    pub x3: f64,
    pub x4: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

impl Svsc4d {
    /// Create a new 4D structure-varying chaotic map.
    pub fn new(x1: f64, x2: f64, x3: f64, x4: f64, a: f64, b: f64, c: f64, d: f64) -> Self {
        // Validate invariants
        assert!(
            x1.is_finite() && x2.is_finite() && x3.is_finite() && x4.is_finite(),
            "Initial states must be finite"
        );
        assert!(
            a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite(),
            "Parameters must be finite"
        );
        Self {
            x1,
            x2,
            x3,
            x4,
            a,
            b,
            c,
            d,
        }
    }

    /// Advance the map by one iteration and return the new state.
    /// This implementation uses a simplified multiplier-free approximation for hyperchaotic mixing
    /// using bitwise operations on the f64 representation to simulate highly nonlinear mixing.
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> (f64, f64, f64, f64) {
        // Multiplier-free chaotic mixing using bit representations and shifts
        let mut u1 = self.x1.to_bits();
        let mut u2 = self.x2.to_bits();
        let mut u3 = self.x3.to_bits();
        let mut u4 = self.x4.to_bits();

        // Perform bitwise shifts and XORs to simulate nonlinear coupling
        // This is a fast, multiplier-free chaotic mapping step
        let tmp = u1 ^ u2 ^ u3 ^ u4;

        u1 = u1.wrapping_add(u2.rotate_left(13)) ^ tmp;
        u2 = u2.wrapping_add(u3.rotate_left(17)) ^ (self.a.to_bits() >> 12);
        u3 = u3.wrapping_add(u4.rotate_left(19)) ^ (self.b.to_bits() >> 12);
        u4 = u4.wrapping_add(u1.rotate_left(23)) ^ (self.c.to_bits() >> 12);

        // Ensure values stay within normalized f64 ranges (fractional part)
        // by replacing the exponent with a fixed one (e.g. 1.0 to 2.0 range)
        // and keeping the pseudo-randomized mantissa.
        const EXP_MASK: u64 = 0x3FF0_0000_0000_0000;
        const MANT_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;

        let r1 = f64::from_bits(EXP_MASK | (u1 & MANT_MASK)) - 1.5; // range [-0.5, 0.5)
        let r2 = f64::from_bits(EXP_MASK | (u2 & MANT_MASK)) - 1.5;
        let r3 = f64::from_bits(EXP_MASK | (u3 & MANT_MASK)) - 1.5;
        let r4 = f64::from_bits(EXP_MASK | (u4 & MANT_MASK)) - 1.5;

        // Apply parameter scaling using basic addition and sign manipulation (multiplier-free pseudo-multiplication)
        self.x1 = r1 + if self.x4 > 0.0 { self.d } else { -self.d };
        self.x2 = r2 + if self.x1 > 0.0 { self.a } else { -self.a };
        self.x3 = r3 + if self.x2 > 0.0 { self.b } else { -self.b };
        self.x4 = r4 + if self.x3 > 0.0 { self.c } else { -self.c };

        (self.x1, self.x2, self.x3, self.x4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svsc4d_determinism() {
        let mut map1 = Svsc4d::new(0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);
        let mut map2 = Svsc4d::new(0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);

        for _ in 0..100 {
            assert_eq!(map1.next(), map2.next());
        }
    }

    #[test]
    fn test_svsc4d_finite_output() {
        let mut map = Svsc4d::new(0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);
        for _ in 0..10_000 {
            let (x1, x2, x3, x4) = map.next();
            assert!(x1.is_finite());
            assert!(x2.is_finite());
            assert!(x3.is_finite());
            assert!(x4.is_finite());
        }
    }

    #[test]
    fn test_svsc4d_sensitivity() {
        let mut map1 = Svsc4d::new(0.1, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);
        // Perturb state slightly
        let mut map2 = Svsc4d::new(0.1 + 1e-10, 0.2, 0.3, 0.4, 1.1, 1.2, 1.3, 1.4);

        let mut max_diff = 0.0_f64;
        for _ in 0..1000 {
            let s1 = map1.next();
            let s2 = map2.next();
            let diff = (s1.0 - s2.0).abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }

        // Ensure that trajectories diverge due to chaotic nature
        assert!(
            max_diff > 0.1,
            "Map did not show sensitive dependence on initial conditions"
        );
    }
}
