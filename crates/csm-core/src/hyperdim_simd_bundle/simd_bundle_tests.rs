#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn bundle_block_avx2_bhvec_correctness() {
    if std::is_x86_feature_detected!("avx2") {
        use crate::hyperdim::BHVec10240;
        let vectors: Vec<BHVec10240> = (0..10u64).map(BHVec10240::new_seeded).collect();
        let refs: Vec<&BHVec10240> = vectors.iter().collect();
        let threshold = vectors.len() / 2 + 1;
        let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
        let simd_res = unsafe { bundle_block_avx2_bhvec(&refs, threshold, num_planes) };
        let mut expected = [0u64; 160];
        for i in 0..160 {
            let mut planes = [0u64; 64];
            for v in &vectors {
                let mut carry = v.bits[i];
                for p in 0..num_planes {
                    let next_carry = planes[p] & carry;
                    planes[p] ^= carry;
                    carry = next_carry;
                    if carry == 0 {
                        break;
                    }
                }
            }
            let (mut current_eq, mut current_gt) = (!0u64, 0u64);
            for p in (0..num_planes).rev() {
                if ((threshold >> p) & 1) == 1 {
                    current_eq &= planes[p];
                } else {
                    current_gt |= current_eq & planes[p];
                    current_eq &= !planes[p];
                }
            }
            expected[i] = current_gt | current_eq;
        }
        assert_eq!(simd_res, expected);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "parallel",
    target_arch = "x86_64"
))]
#[test]
fn bundle_parallel_avx2_bhvec_correctness() {
    if std::is_x86_feature_detected!("avx2") {
        use crate::hyperdim::BHVec10240;
        // Use 300 vectors to cross-validate parallel execution and Rayon chunking
        let vectors: Vec<BHVec10240> = (0..300u64).map(BHVec10240::new_seeded).collect();
        let refs: Vec<&BHVec10240> = vectors.iter().collect();
        let threshold = vectors.len() / 2 + 1;
        let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
        let simd_res = bundle_parallel_avx2_bhvec(&refs, threshold, num_planes).unwrap();
        let mut expected = [0u64; 160];
        for i in 0..160 {
            let mut planes = [0u64; 64];
            for v in &vectors {
                let mut carry = v.bits[i];
                for p in 0..num_planes {
                    let next_carry = planes[p] & carry;
                    planes[p] ^= carry;
                    carry = next_carry;
                    if carry == 0 {
                        break;
                    }
                }
            }
            let (mut current_eq, mut current_gt) = (!0u64, 0u64);
            for p in (0..num_planes).rev() {
                if ((threshold >> p) & 1) == 1 {
                    current_eq &= planes[p];
                } else {
                    current_gt |= current_eq & planes[p];
                    current_eq &= !planes[p];
                }
            }
            expected[i] = current_gt | current_eq;
        }
        assert_eq!(simd_res, expected);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
#[test]
fn bundle_block_avx2_correctness() {
    if std::arch::is_x86_feature_detected!("avx2") {
        use crate::hyperdim::HVec10240;
        let vectors: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
        let threshold = vectors.len() / 2 + 1;
        let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
        let simd_res = unsafe { bundle_block_avx2(&vectors, threshold, num_planes) };
        let mut expected = [0u128; 80];
        for i in 0..80 {
            let mut planes = [0u128; 64];
            for v in &vectors {
                let mut carry = v.data[i];
                for p in 0..num_planes {
                    let next_carry = planes[p] & carry;
                    planes[p] ^= carry;
                    carry = next_carry;
                    if carry == 0 {
                        break;
                    }
                }
            }
            let (mut current_eq, mut current_gt) = (!0u128, 0u128);
            for p in (0..num_planes).rev() {
                if ((threshold >> p) & 1) == 1 {
                    current_eq &= planes[p];
                } else {
                    current_gt |= current_eq & planes[p];
                    current_eq &= !planes[p];
                }
            }
            expected[i] = current_gt | current_eq;
        }
        assert_eq!(simd_res, expected);
    }
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[test]
fn bundle_block_neon_correctness() {
    use crate::hyperdim::HVec10240;
    let vectors: Vec<HVec10240> = (0..10u64).map(HVec10240::new_seeded).collect();
    let threshold = vectors.len() / 2 + 1;
    let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
    let simd_res = unsafe { bundle_block_neon(&vectors, threshold, num_planes) };
    let mut expected = [0u128; 80];
    for i in 0..80 {
        let mut planes = [0u128; 64];
        for v in &vectors {
            let mut carry = v.data[i];
            for p in 0..num_planes {
                let next_carry = planes[p] & carry;
                planes[p] ^= carry;
                carry = next_carry;
                if carry == 0 {
                    break;
                }
            }
        }
        let (mut current_eq, mut current_gt) = (!0u128, 0u128);
        for p in (0..num_planes).rev() {
            if ((threshold >> p) & 1) == 1 {
                current_eq &= planes[p];
            } else {
                current_gt |= current_eq & planes[p];
                current_eq &= !planes[p];
            }
        }
        expected[i] = current_gt | current_eq;
    }
    assert_eq!(simd_res, expected);
}

#[cfg(all(not(target_arch = "wasm32"), target_arch = "aarch64"))]
#[test]
fn bundle_block_neon_bhvec_correctness() {
    use crate::hyperdim::BHVec10240;
    let vectors: Vec<BHVec10240> = (0..10u64).map(BHVec10240::new_seeded).collect();
    let refs: Vec<&BHVec10240> = vectors.iter().collect();
    let threshold = vectors.len() / 2 + 1;
    let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
    let simd_res = unsafe { bundle_block_neon_bhvec(&refs, threshold, num_planes) };
    let mut expected = [0u64; 160];
    for i in 0..160 {
        let mut planes = [0u64; 64];
        for v in &vectors {
            let mut carry = v.bits[i];
            for p in 0..num_planes {
                let next_carry = planes[p] & carry;
                planes[p] ^= carry;
                carry = next_carry;
                if carry == 0 {
                    break;
                }
            }
        }
        let (mut current_eq, mut current_gt) = (!0u64, 0u64);
        for p in (0..num_planes).rev() {
            if ((threshold >> p) & 1) == 1 {
                current_eq &= planes[p];
            } else {
                current_gt |= current_eq & planes[p];
                current_eq &= !planes[p];
            }
        }
        expected[i] = current_gt | current_eq;
    }
    assert_eq!(simd_res, expected);
}

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "parallel",
    target_arch = "aarch64"
))]
#[test]
fn bundle_parallel_neon_bhvec_correctness() {
    use crate::hyperdim::BHVec10240;
    // Use 300 vectors to cross-validate parallel execution and Rayon chunking
    let vectors: Vec<BHVec10240> = (0..300u64).map(BHVec10240::new_seeded).collect();
    let refs: Vec<&BHVec10240> = vectors.iter().collect();
    let threshold = vectors.len() / 2 + 1;
    let num_planes = (usize::BITS - vectors.len().leading_zeros()) as usize;
    let simd_res = bundle_parallel_neon_bhvec(&refs, threshold, num_planes);
    let mut expected = [0u64; 160];
    for i in 0..160 {
        let mut planes = [0u64; 64];
        for v in &vectors {
            let mut carry = v.bits[i];
            for p in 0..num_planes {
                let next_carry = planes[p] & carry;
                planes[p] ^= carry;
                carry = next_carry;
                if carry == 0 {
                    break;
                }
            }
        }
        let (mut current_eq, mut current_gt) = (!0u64, 0u64);
        for p in (0..num_planes).rev() {
            if ((threshold >> p) & 1) == 1 {
                current_eq &= planes[p];
            } else {
                current_gt |= current_eq & planes[p];
                current_eq &= !planes[p];
            }
        }
        expected[i] = current_gt | current_eq;
    }
    assert_eq!(simd_res, expected);
}
