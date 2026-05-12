use crate::hyperdim::HVec10240;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

/// Batch similarity computation tuned for cache locality and Rayon overhead.
pub fn batch_cosine_similarity(query: &HVec10240, candidates: &[HVec10240]) -> Vec<f32> {
    #[cfg(all(not(target_arch = "wasm32"), target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            let mut distances = vec![0u32; candidates.len()];
            unsafe {
                crate::hyperdim_simd::batch_hamming_distance_avx2(
                    &query.data,
                    candidates,
                    &mut distances,
                );
            }
            return distances
                .into_iter()
                .map(|dist| 1.0 - (dist as f32 / 5120.0))
                .collect();
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
    {
        const CHUNK_SIZE: usize = 512;
        let mut results = vec![0.0f32; candidates.len()];
        candidates
            .par_chunks(CHUNK_SIZE)
            .zip(results.par_chunks_mut(CHUNK_SIZE))
            .for_each(|(chunk, out)| process_chunk(query, chunk, out));
        results
    }

    #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
    {
        candidates
            .iter()
            .map(|candidate| query.cosine_similarity(candidate))
            .collect()
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
fn process_chunk(query: &HVec10240, chunk: &[HVec10240], out: &mut [f32]) {
    for (slot, candidate) in out.iter_mut().zip(chunk.iter()) {
        *slot = query.cosine_similarity(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_batch_returns_empty_vec() {
        let query = HVec10240::random();
        let results = batch_cosine_similarity(&query, &[]);
        assert!(results.is_empty());
    }

    #[test]
    fn batch_matches_scalar_results() {
        let query = HVec10240::random();
        let candidates: Vec<_> = (0..11).map(|_| HVec10240::random()).collect();
        let batch_results = batch_cosine_similarity(&query, &candidates);
        let scalar_results: Vec<_> = candidates
            .iter()
            .map(|c| query.cosine_similarity(c))
            .collect();
        assert_eq!(batch_results.len(), scalar_results.len());
        for (lhs, rhs) in batch_results.iter().zip(scalar_results.iter()) {
            assert!((lhs - rhs).abs() < f32::EPSILON);
        }
    }
}
