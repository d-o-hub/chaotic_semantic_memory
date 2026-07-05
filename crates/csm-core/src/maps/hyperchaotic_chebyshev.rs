#![cfg(feature = "chaotic-hashing")]
//! 2D Chebyshev-Logistic Hyperchaotic Map re-export.

pub use csm_chaos::ChebyshevLogistic2d;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chebyshev_logistic_reexport() {
        let mut map = ChebyshevLogistic2d::new(0.123, -0.456, 4.0, 4);
        let v = map.next_value();
        assert!(v >= 0.0 && v < 1.0);
    }
}
