#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! Exact search via linear scan.

// Casts are intentional for similarity math

use std::collections::HashMap;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use crate::index::{AnnIndex, IndexStats};
use crate::singularity::Concept;
use csm_core_lib::error::Result;
use csm_core_lib::hyperdim::{HVec10240, Hypervector};

/// Exact search via linear scan.
#[derive(Debug)]
pub struct BruteForce<H: Hypervector = HVec10240> {
    indices: Vec<String>,
    vectors: Vec<H>,
    id_to_index: HashMap<String, usize>,
}

impl<H: Hypervector> Default for BruteForce<H> {
    fn default() -> Self {
        Self {
            indices: Vec::new(),
            vectors: Vec::new(),
            id_to_index: HashMap::new(),
        }
    }
}

impl<H: Hypervector> BruteForce<H> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<H: Hypervector + 'static> AnnIndex<H> for BruteForce<H> {
    fn insert(&mut self, id: String, vec: &H) -> Result<()> {
        if let Some(&idx) = self.id_to_index.get(&id) {
            self.vectors[idx] = *vec;
        } else {
            let idx = self.indices.len();
            self.id_to_index.insert(id.clone(), idx);
            self.indices.push(id);
            self.vectors.push(*vec);
        }
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if let Some(idx) = self.id_to_index.remove(id) {
            self.indices.swap_remove(idx);
            let _ = self.vectors.swap_remove(idx);
            if idx < self.indices.len() {
                let swapped_id = &self.indices[idx];
                self.id_to_index.insert(swapped_id.clone(), idx);
            }
        }
        Ok(())
    }

    fn search(&self, query: &H, top_k: usize) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.indices.is_empty() {
            return Ok(Vec::new());
        }

        // Algorithmic Optimization: Parallelize O(N) similarity scan via Rayon.
        // Hamming distance calculations for 10240-bit vectors are CPU-bound and highly
        // parallelizable, allowing for significant throughput gains on multi-core systems.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(usize, u32)> = self
            .vectors
            .par_iter()
            .enumerate()
            .with_min_len(128)
            .map(|(idx, v)| (idx, query.hamming_distance(v)))
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(usize, u32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(idx, v)| (idx, query.hamming_distance(v)))
            .collect();

        if scores.len() <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(idx, dist)| {
                let similarity = 1.0 - (dist as f32 / 5120.0);
                (self.indices[idx].clone(), similarity)
            })
            .collect();

        Ok(results)
    }

    fn search_filtered(
        &self,
        query: &H,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
        concepts: &HashMap<String, Concept<H>>,
    ) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.indices.is_empty() {
            return Ok(Vec::new());
        }

        // Algorithmic Optimization: Parallelize filtered similarity scan via Rayon.
        // This distributes both the metadata predicate matching and the Hamming distance
        // calculations across available CPU cores.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(usize, u32)> = self
            .indices
            .par_iter()
            .enumerate()
            .filter(|(_, id)| {
                concepts
                    .get(*id)
                    .is_some_and(|c| filter.matches(&c.metadata))
            })
            .map(|(idx, _)| (idx, query.hamming_distance(&self.vectors[idx])))
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(usize, u32)> = self
            .indices
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                concepts
                    .get(*id)
                    .is_some_and(|c| filter.matches(&c.metadata))
            })
            .map(|(idx, _)| (idx, query.hamming_distance(&self.vectors[idx])))
            .collect();

        if scores.len() <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(idx, dist)| {
                let similarity = 1.0 - (dist as f32 / 5120.0);
                (self.indices[idx].clone(), similarity)
            })
            .collect();

        Ok(results)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept<H>>) -> Result<()> {
        self.indices.clear();
        self.vectors.clear();
        self.id_to_index.clear();

        for (id, concept) in concepts {
            self.insert(id.clone(), &concept.vector)?;
        }
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            backend: "BruteForce".to_string(),
            count: self.indices.len(),
            memory_usage_bytes: self.indices.len()
                * (std::mem::size_of::<String>() + std::mem::size_of::<H>() + 16),
        }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        // BruteForce doesn't need to serialize its state independently as it can be rebuilt from concepts.
        // However, for trait consistency, we return an empty vec or a simple marker.
        Ok(Vec::new())
    }

    fn deserialize(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}
