#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! LSH ANN index backend (ADR-0068).

// Casts are intentional for similarity math

use rand::RngExt;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
use rayon::prelude::*;

use csm_core::Result;
use csm_core::hyperdim::HVec10240;
use crate::index::{AnnIndex, IndexStats};
use csm_memory::singularity::Concept;

/// Locality-Sensitive Hashing (LSH) for hypervectors using bit-sampling.
#[derive(Debug, Serialize, Deserialize)]
pub struct LshIndex {
    num_tables: usize,
    hash_bits: usize,
    tables: Vec<HashMap<u64, Vec<String>>>,
    projections: Vec<Vec<usize>>, // indices of bits to sample for each table
    concepts: HashMap<String, HVec10240>,
}

impl LshIndex {
    pub fn new(num_tables: usize, hash_bits: usize) -> Result<Self> {
        // #9: Reject zero-table configurations.
        if num_tables == 0 {
            return Err(csm_core::MemoryError::InvalidInput {
                field: "num_tables".to_string(),
                reason: "num_tables must be greater than zero".to_string(),
            });
        }
        // Safety check: prevent hash_bits > 64 since we use u64 hashes
        let hash_bits = hash_bits.min(64);
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut projections = Vec::with_capacity(num_tables);
        let mut tables = Vec::with_capacity(num_tables);

        for _ in 0..num_tables {
            let mut bits = Vec::with_capacity(hash_bits);
            for _ in 0..hash_bits {
                bits.push(rng.random_range(0..HVec10240::DIMENSION));
            }
            projections.push(bits);
            tables.push(HashMap::new());
        }

        Ok(Self {
            num_tables,
            hash_bits,
            tables,
            projections,
            concepts: HashMap::new(),
        })
    }

    fn compute_hash(&self, vec: &HVec10240, table_idx: usize) -> u64 {
        let mut hash = 0u64;
        let bits = &self.projections[table_idx];
        for (i, &bit_pos) in bits.iter().enumerate() {
            let word = bit_pos / 128;
            let bit = bit_pos % 128;
            if (vec.data[word] & (1u128 << bit)) != 0 {
                hash |= 1u64 << i;
            }
        }
        hash
    }
}

impl AnnIndex for LshIndex {
    fn insert(&mut self, id: String, vec: &HVec10240) -> Result<()> {
        if self.concepts.contains_key(&id) {
            self.delete(&id)?;
        }

        for i in 0..self.num_tables {
            let hash = self.compute_hash(vec, i);
            self.tables[i].entry(hash).or_default().push(id.clone());
        }
        self.concepts.insert(id, *vec);
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if let Some(vec) = self.concepts.remove(id) {
            for i in 0..self.num_tables {
                let hash = self.compute_hash(&vec, i);
                if let Some(bucket) = self.tables[i].get_mut(&hash) {
                    bucket.retain(|x| x != id);
                }
            }
        }
        Ok(())
    }

    fn search(&self, query: &HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.concepts.is_empty() {
            return Ok(Vec::new());
        }

        let mut candidates = HashMap::new();
        for i in 0..self.num_tables {
            let hash = self.compute_hash(query, i);
            if let Some(bucket) = self.tables[i].get(&hash) {
                for id in bucket {
                    candidates.entry(id).or_insert(());
                }
            }
        }

        // Algorithmic Optimization: Parallelize candidate re-ranking via Rayon.
        // This accelerates the exhaustive similarity check of the candidate
        // set retrieved from LSH buckets.
        // Optimized: Uses integer Hamming distance and references to avoid
        // expensive string allocations in the parallel loop.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(&String, u32)> = candidates
            .keys()
            .par_bridge()
            .filter_map(|id| {
                self.concepts
                    .get(*id)
                    .map(|vec| (*id, query.hamming_distance(vec)))
            })
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(&String, u32)> = candidates
            .keys()
            .filter_map(|id| {
                self.concepts
                    .get(*id)
                    .map(|vec| (*id, query.hamming_distance(vec)))
            })
            .collect();

        // Optimized: Use O(N) partial selection instead of O(N log N) full sort.
        if scores.len() <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let results = scores
            .into_iter()
            .map(|(id, dist)| (id.clone(), 1.0 - (dist as f32 / 5120.0)))
            .collect();
        Ok(results)
    }

    fn search_filtered(
        &self,
        query: &HVec10240,
        top_k: usize,
        filter: &csm_memory::metadata_filter::MetadataFilter,
        concepts: &HashMap<String, Concept>,
    ) -> Result<Vec<(String, f32)>> {
        if top_k == 0 || self.concepts.is_empty() {
            return Ok(Vec::new());
        }

        let mut candidates = HashMap::new();
        for i in 0..self.num_tables {
            let hash = self.compute_hash(query, i);
            if let Some(bucket) = self.tables[i].get(&hash) {
                for id in bucket {
                    if let Some(concept) = concepts.get(id) {
                        if filter.matches(&concept.metadata) {
                            candidates.entry(id).or_insert(());
                        }
                    }
                }
            }
        }

        // Algorithmic Optimization: Parallelize candidate re-ranking via Rayon.
        // Optimized: Uses integer Hamming distance and references to avoid
        // expensive string allocations in the parallel loop.
        #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
        let mut scores: Vec<(&String, u32)> = candidates
            .keys()
            .par_bridge()
            .filter_map(|id| {
                self.concepts
                    .get(*id)
                    .map(|vec| (*id, query.hamming_distance(vec)))
            })
            .collect();

        #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
        let mut scores: Vec<(&String, u32)> = candidates
            .keys()
            .filter_map(|id| {
                self.concepts
                    .get(*id)
                    .map(|vec| (*id, query.hamming_distance(vec)))
            })
            .collect();

        // Optimized: Use O(N) partial selection instead of O(N log N) full sort.
        if scores.len() <= top_k {
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        } else {
            scores.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
            scores.truncate(top_k);
            scores.sort_unstable_by_key(|&(_, dist)| dist);
        }

        let final_scores: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(id, dist)| (id.clone(), 1.0 - (dist as f32 / 5120.0)))
            .collect();

        // Fallback for correctness: if we have few candidates, check all filtered concepts
        if final_scores.len() < top_k {
            // Algorithmic Optimization: Parallelize the full-scan fallback via Rayon.
            // This prevents a performance cliff when LSH buckets fail to yield enough
            // valid candidates for a specific filter.
            #[cfg(all(not(target_arch = "wasm32"), feature = "parallel"))]
            let mut all_filtered: Vec<(&String, u32)> = concepts
                .par_iter()
                .filter(|(_, c)| filter.matches(&c.metadata))
                .map(|(id, c)| (id, query.hamming_distance(&c.vector)))
                .collect();

            #[cfg(any(target_arch = "wasm32", not(feature = "parallel")))]
            let mut all_filtered: Vec<(&String, u32)> = concepts
                .iter()
                .filter(|(_, c)| filter.matches(&c.metadata))
                .map(|(id, c)| (id, query.hamming_distance(&c.vector)))
                .collect();

            if all_filtered.len() <= top_k {
                all_filtered.sort_unstable_by_key(|&(_, dist)| dist);
            } else {
                all_filtered.select_nth_unstable_by(top_k - 1, |a, b| a.1.cmp(&b.1));
                all_filtered.truncate(top_k);
                all_filtered.sort_unstable_by_key(|&(_, dist)| dist);
            }

            let results = all_filtered
                .into_iter()
                .map(|(id, dist)| (id.clone(), 1.0 - (dist as f32 / 5120.0)))
                .collect();
            return Ok(results);
        }

        Ok(final_scores)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept>) -> Result<()> {
        for table in &mut self.tables {
            table.clear();
        }
        self.concepts.clear();

        for (id, concept) in concepts {
            self.insert(id.clone(), &concept.vector)?;
        }
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        let mut total_buckets = 0;
        for table in &self.tables {
            total_buckets += table.len();
        }

        IndexStats {
            backend: "LSH".to_string(),
            count: self.concepts.len(),
            memory_usage_bytes: self.concepts.len()
                * (std::mem::size_of::<String>() + std::mem::size_of::<HVec10240>())
                + total_buckets * std::mem::size_of::<Vec<String>>(),
        }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| {
            csm_core::MemoryError::Persistence(format!("Serialization error: {}", e))
        })
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        let decoded: Self = bincode::deserialize(data).map_err(|e| {
            csm_core::MemoryError::Persistence(format!("Deserialization error: {}", e))
        })?;
        *self = decoded;
        Ok(())
    }
}
