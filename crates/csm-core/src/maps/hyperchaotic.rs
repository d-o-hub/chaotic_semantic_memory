#![cfg(feature = "chaotic-hashing")]
//! 2D Sine-Logistic Hyperchaotic Map (2D-SLHM) re-export.

pub use csm_chaos::Slhm2d;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slhm2d_reexport() {
        let mut map = Slhm2d::new(0.123, 0.456, 0.99);
        let v = map.next_value();
        assert!(v >= 0.0 && v < 1.0);
    }
}
