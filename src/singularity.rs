use std::collections::HashMap;

use rayon::prelude::*;

use crate::hyperdim::HVec10240;

#[derive(Clone, Default)]
pub struct SingularityCore {
    pub concepts: HashMap<String, HVec10240>,
}

impl SingularityCore {
    pub fn inject_concept(&mut self, name: &str, hvec: HVec10240) -> f64 {
        let novelty = self
            .concepts
            .values()
            .map(|v| 1.0 - v.cosine_similarity(&hvec).abs())
            .fold(1.0, f64::min);
        self.concepts.insert(name.to_owned(), hvec);
        novelty
    }

    pub fn probe(&self, seed: HVec10240, top_k: usize) -> Vec<(String, f64)> {
        let mut sims: Vec<(String, f64)> = self
            .concepts
            .par_iter()
            .map(|(name, hv)| (name.clone(), hv.cosine_similarity(&seed)))
            .collect();
        sims.par_sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sims.truncate(top_k);
        sims
    }
}
