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
use crate::error::{MemoryError, Result};
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
#[derive(Clone)]
struct HammingDist;

#[cfg(feature = "ann-hnsw")]
impl Distance<HVec10240> for HammingDist {
    fn eval(&self, va: &[HVec10240], vb: &[HVec10240]) -> f32 {
        va[0].hamming_distance(&vb[0]) as f32
    }
}

#[cfg(feature = "ann-hnsw")]
pub struct HnswIndex {
    hnsw: Hnsw<'static, HVec10240, HammingDist>,
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    config: HnswData,
}

#[cfg(feature = "ann-hnsw")]
impl HnswIndex {
    pub fn new(m: usize, ef_construction: usize, ef_search: usize) -> Self {
        // ADR-0068: Default to 1M elements to support scale goal
        let hnsw = Hnsw::new(m, 1_000_000, 16, ef_construction, HammingDist);
        Self {
            hnsw,
            id_to_idx: HashMap::new(),
            idx_to_id: HashMap::new(),
            config: HnswData {
                m,
                ef_construction,
                ef_search,
            },
        }
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
#[derive(Serialize, Deserialize)]
struct HnswPersistenceMeta {
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
    graph_len: usize,
}

#[cfg(feature = "ann-hnsw")]
impl AnnIndex for HnswIndex {
    fn insert(&mut self, id: String, vec: &HVec10240) -> Result<()> {
        let idx = self.id_to_idx.len();
        self.hnsw.insert((std::slice::from_ref(vec), idx));
        self.id_to_idx.insert(id.clone(), idx);
        self.idx_to_id.insert(idx, id);
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if let Some(idx) = self.id_to_idx.remove(id) {
            self.idx_to_id.remove(&idx);
        }
        Ok(())
    }

    fn search(&self, query: &HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
        let results = self
            .hnsw
            .search(std::slice::from_ref(query), top_k, self.config.ef_search);

        let mut final_results = Vec::with_capacity(results.len());
        for neighbor in results {
            if let Some(id) = self.idx_to_id.get(&neighbor.d_id) {
                let similarity = 1.0 - (neighbor.distance / 5120.0);
                final_results.push((id.clone(), similarity));
            }
        }
        Ok(final_results)
    }

    fn search_filtered(
        &self,
        query: &HVec10240,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
        concepts: &HashMap<String, Concept>,
    ) -> Result<Vec<(String, f32)>> {
        // HNSW doesn't support pre-filtering natively.
        // We'll search a larger set and post-filter.
        let expanded_k = top_k * 5;
        let results = self.hnsw.search(
            std::slice::from_ref(query),
            expanded_k,
            self.config.ef_search,
        );

        let mut filtered_results = Vec::new();
        for neighbor in results {
            if let Some(id) = self.idx_to_id.get(&neighbor.d_id) {
                if let Some(concept) = concepts.get(id) {
                    if filter.matches(&concept.metadata) {
                        let similarity = 1.0 - (neighbor.distance / 5120.0);
                        filtered_results.push((id.clone(), similarity));
                        if filtered_results.len() >= top_k {
                            break;
                        }
                    }
                }
            }
        }

        // If we didn't get enough results, we fall back to a full scan of the filtered set
        // to ensure we don't return fewer than top_k if they exist.
        if filtered_results.len() < top_k {
            let mut all_filtered: Vec<(String, f32)> = concepts
                .iter()
                .filter(|(_, c)| filter.matches(&c.metadata))
                .map(|(id, c)| {
                    (
                        id.clone(),
                        1.0 - (query.hamming_distance(&c.vector) as f32 / 5120.0),
                    )
                })
                .collect();

            all_filtered.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
            });
            all_filtered.truncate(top_k);
            return Ok(all_filtered);
        }

        Ok(filtered_results)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept>) -> Result<()> {
        self.hnsw = Hnsw::new(
            self.config.m,
            concepts.len().max(100),
            16,
            self.config.ef_construction,
            HammingDist,
        );
        self.id_to_idx.clear();
        self.idx_to_id.clear();

        for (id, concept) in concepts {
            self.insert(id.clone(), &concept.vector)?;
        }
        Ok(())
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
        use std::fs;

        let temp_dir = std::env::temp_dir().join(format!("csm_hnsw_{}", rand::random::<u64>()));
        fs::create_dir_all(&temp_dir).map_err(MemoryError::Io)?;

        self.hnsw
            .file_dump(&temp_dir, "index")
            .map_err(|e| MemoryError::database(format!("HNSW dump failed: {}", e)))?;

        let data_path = temp_dir.join("index.hnsw.data");
        let graph_path = temp_dir.join("index.hnsw.graph");

        let data_bytes = fs::read(data_path).map_err(MemoryError::Io)?;
        let graph_bytes = fs::read(graph_path).map_err(MemoryError::Io)?;

        let meta = HnswPersistenceMeta {
            id_to_idx: self.id_to_idx.clone(),
            idx_to_id: self.idx_to_id.clone(),
            m: self.config.m,
            ef_construction: self.config.ef_construction,
            ef_search: self.config.ef_search,
            graph_len: graph_bytes.len(),
        };

        let mut payload = bincode::serialize(&meta)
            .map_err(|e| MemoryError::database(format!("Bincode fail: {}", e)))?;
        payload.extend_from_slice(&data_bytes);
        payload.extend_from_slice(&graph_bytes);

        // Cleanup temp files
        let _ = fs::remove_dir_all(temp_dir);

        Ok(payload)
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        use std::fs;

        if data.is_empty() {
            return Ok(());
        }

        let meta_size = std::mem::size_of::<HnswPersistenceMeta>();
        if data.len() < meta_size {
            return Err(MemoryError::database("Index data too small".to_string()));
        }

        // We don't know the exact size of the bincode header, so we let bincode tell us
        let meta: HnswPersistenceMeta = bincode::deserialize(data)
            .map_err(|e| MemoryError::database(format!("Bincode deserialize fail: {}", e)))?;

        let meta_serialized_size: usize = bincode::serialized_size(&meta)
            .map_err(|e| MemoryError::database(format!("Bincode size fail: {}", e)))?
            .try_into()
            .map_err(|e| MemoryError::database(format!("Bincode size truncation: {}", e)))?;

        let graph_start = data.len() - meta.graph_len;
        let data_start = meta_serialized_size;
        let data_bytes = &data[data_start..graph_start];
        let graph_bytes = &data[graph_start..];

        let temp_dir =
            std::env::temp_dir().join(format!("csm_hnsw_load_{}", rand::random::<u64>()));
        fs::create_dir_all(&temp_dir).map_err(MemoryError::Io)?;

        fs::write(temp_dir.join("index.hnsw.data"), data_bytes).map_err(MemoryError::Io)?;
        fs::write(temp_dir.join("index.hnsw.graph"), graph_bytes).map_err(MemoryError::Io)?;

        let loader = HnswIo::new(&temp_dir, "index");
        let hnsw = loader
            .load_hnsw_with_dist::<HVec10240, HammingDist>(HammingDist)
            .map_err(|e| MemoryError::database(format!("HNSW load failed: {}", e)))?;

        // SAFETY: We are not using mmap (datamap_opt is false), so the Hnsw struct
        // only contains owned data (PointData::V variants). The lifetime 'b is
        // only needed for PointData::S (mmap), which we don't use.
        let static_hnsw: Hnsw<'static, HVec10240, HammingDist> =
            unsafe { std::mem::transmute(hnsw) };

        self.hnsw = static_hnsw;
        self.id_to_idx = meta.id_to_idx;
        self.idx_to_id = meta.idx_to_id;
        self.config.m = meta.m;
        self.config.ef_construction = meta.ef_construction;
        self.config.ef_search = meta.ef_search;

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);

        Ok(())
    }
}
