//! HNSW ANN index backend (ADR-0068).

// Casts are intentional for similarity math
#![allow(clippy::cast_precision_loss)]

#[cfg(feature = "ann-hnsw")]
use hnsw_rs::prelude::*;
#[cfg(feature = "ann-hnsw")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ann-hnsw")]
use std::collections::HashMap;

#[cfg(feature = "ann-hnsw")]
use crate::error::Result;
#[cfg(feature = "ann-hnsw")]
use crate::hyperdim::HVec10240;
#[cfg(feature = "ann-hnsw")]
use crate::index::{AnnIndex, IndexStats};
#[cfg(feature = "ann-hnsw")]
use crate::singularity::Concept;

#[cfg(feature = "ann-hnsw")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswData {
    m: usize,
    ef_construction: usize,
    ef_search: usize,
}

#[cfg(feature = "ann-hnsw")]
#[derive(Clone, Default)]
struct HammingDist;

#[cfg(feature = "ann-hnsw")]
impl Distance<[u128; 80]> for HammingDist {
    fn eval(&self, va: &[[u128; 80]], vb: &[[u128; 80]]) -> f32 {
        let mut dist = 0u32;
        let va = va[0];
        let vb = vb[0];
        for i in 0..80 {
            dist += (va[i] ^ vb[i]).count_ones();
        }
        dist as f32
    }
}

#[cfg(feature = "ann-hnsw")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswSnapshot {
    config: HnswData,
    vectors: HashMap<String, HVec10240>,
}

#[cfg(feature = "ann-hnsw")]
pub struct HnswIndex {
    hnsw: Hnsw<'static, [u128; 80], HammingDist>,
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    vectors: HashMap<String, HVec10240>,
    config: HnswData,
}

#[cfg(feature = "ann-hnsw")]
impl HnswIndex {
    fn new_hnsw(
        config: &HnswData,
        expected_count: usize,
    ) -> Hnsw<'static, [u128; 80], HammingDist> {
        Hnsw::new(
            config.m,
            expected_count.max(100),
            16,
            config.ef_construction,
            HammingDist,
        )
    }

    pub fn new(m: usize, ef_construction: usize, ef_search: usize) -> Self {
        let config = HnswData {
            m,
            ef_construction,
            ef_search,
        };

        // ADR-0068: Default to 1M elements to support scale goal
        let hnsw = Self::new_hnsw(&config, 1_000_000);
        Self {
            hnsw,
            id_to_idx: HashMap::new(),
            idx_to_id: HashMap::new(),
            vectors: HashMap::new(),
            config,
        }
    }

    fn rebuild_from_vectors(&mut self) -> Result<()> {
        self.hnsw = Self::new_hnsw(&self.config, self.vectors.len());
        self.id_to_idx.clear();
        self.idx_to_id.clear();

        let mut entries: Vec<(String, HVec10240)> = self
            .vectors
            .iter()
            .map(|(id, vec)| (id.clone(), *vec))
            .collect();
        entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

        for (idx, (id, vec)) in entries.into_iter().enumerate() {
            self.hnsw.insert((std::slice::from_ref(&vec.data), idx));
            self.id_to_idx.insert(id.clone(), idx);
            self.idx_to_id.insert(idx, id);
        }
        Ok(())
    }
}

#[cfg(feature = "ann-hnsw")]
impl std::fmt::Debug for HnswIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswIndex")
            .field("config", &self.config)
            .field("count", &self.id_to_idx.len())
            .finish()
    }
}

#[cfg(feature = "ann-hnsw")]
impl AnnIndex for HnswIndex {
    fn insert(&mut self, id: String, vec: &HVec10240) -> Result<()> {
        if self.id_to_idx.contains_key(&id) {
            self.vectors.insert(id, *vec);
            return self.rebuild_from_vectors();
        }

        let idx = self.id_to_idx.len();
        self.hnsw.insert((std::slice::from_ref(&vec.data), idx));
        self.vectors.insert(id.clone(), *vec);
        self.id_to_idx.insert(id.clone(), idx);
        self.idx_to_id.insert(idx, id);
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if self.vectors.remove(id).is_some() {
            self.rebuild_from_vectors()?;
        }
        Ok(())
    }

    fn search(&self, query: &HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        let results = self.hnsw.search(
            std::slice::from_ref(&query.data),
            top_k,
            self.config.ef_search,
        );

        let mut final_results = Vec::with_capacity(results.len());
        for neighbor in results {
            if let Some(id) = self.idx_to_id.get(&neighbor.d_id) {
                let similarity = 1.0 - (neighbor.distance / 5120.0);
                final_results.push((id.clone(), similarity));
            }
        }
        Ok(final_results)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept>) -> Result<()> {
        self.vectors.clear();

        for (id, concept) in concepts {
            self.vectors.insert(id.clone(), concept.vector);
        }
        self.rebuild_from_vectors()
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            backend: "HNSW".to_string(),
            count: self.id_to_idx.len(),
            memory_usage_bytes: self.id_to_idx.len()
                * (std::mem::size_of::<String>() + std::mem::size_of::<HVec10240>() + 32),
        }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        let snapshot = HnswSnapshot {
            config: self.config.clone(),
            vectors: self.vectors.clone(),
        };
        bincode::serialize(&snapshot).map_err(|e| {
            crate::error::MemoryError::Persistence(format!("Serialization error: {}", e))
        })
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        let snapshot: HnswSnapshot = bincode::deserialize(data).map_err(|e| {
            crate::error::MemoryError::Persistence(format!("Deserialization error: {}", e))
        })?;
        self.config = snapshot.config;
        self.vectors = snapshot.vectors;
        self.rebuild_from_vectors()
    }
}
