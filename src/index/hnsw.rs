#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! HNSW ANN index backend (ADR-0068).

#[cfg(feature = "ann-hnsw")]
use hnsw_rs::prelude::*;
#[cfg(feature = "ann-hnsw")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ann-hnsw")]
use std::collections::HashMap;

#[cfg(feature = "ann-hnsw")]
use crate::error::{MemoryError, Result};
#[cfg(feature = "ann-hnsw")]
use crate::hyperdim::{HVec10240, Hypervector};
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
struct HammingDist<H: Hypervector>(std::marker::PhantomData<H>);

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector> Distance<H> for HammingDist<H> {
    fn eval(&self, va: &[H], vb: &[H]) -> f32 {
        va[0].hamming_distance(&vb[0]) as f32
    }
}

#[cfg(feature = "ann-hnsw")]
pub struct HnswIndex<H: Hypervector> {
    hnsw: Hnsw<'static, H, HammingDist<H>>,
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    config: HnswData,
    deleted_count: usize,
    _temp_dir: Option<tempfile::TempDir>,
    _loader: Option<HnswIo>,
}

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector + 'static> HnswIndex<H> {
    pub fn new(m: usize, ef_construction: usize, ef_search: usize) -> Result<Self> {
        if m == 0 || m > 256 {
            return Err(MemoryError::InvalidInput { field: "m".to_string(), reason: "m must be 1..256".to_string() });
        }
        let hnsw = Hnsw::new(m, 1_000_000, 16, ef_construction, HammingDist(std::marker::PhantomData));
        Ok(Self {
            hnsw,
            id_to_idx: HashMap::new(),
            idx_to_id: HashMap::new(),
            config: HnswData { m, ef_construction, ef_search },
            deleted_count: 0,
            _temp_dir: None,
            _loader: None,
        })
    }
}

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector + 'static> std::fmt::Debug for HnswIndex<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswIndex")
            .field("config", &self.config)
            .field("count", &self.id_to_idx.len())
            .finish()
    }
}

#[cfg(feature = "ann-hnsw")]
#[derive(Serialize, Deserialize)]
struct HnswPersistenceWrapper {
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    m: usize,
    ef_construction: usize,
    ef_search: usize,
    deleted_count: usize,
    data: Vec<u8>,
    graph: Vec<u8>,
}

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector> AnnIndex<H> for HnswIndex<H> {
    fn insert(&mut self, id: String, vec: &H) -> Result<()> {
        if self.id_to_idx.contains_key(&id) { self.delete(&id)?; }
        let idx = self.hnsw.get_nb_point();
        self.hnsw.insert((std::slice::from_ref(vec), idx));
        self.id_to_idx.insert(id.clone(), idx);
        self.idx_to_id.insert(idx, id);
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        if let Some(idx) = self.id_to_idx.remove(id) {
            self.idx_to_id.remove(&idx);
            self.deleted_count += 1;
        }
        Ok(())
    }

    fn search(&self, query: &H, top_k: usize) -> Result<Vec<(String, f32)>> {
        let expanded_k = top_k + self.deleted_count.min(top_k * 10);
        let results = self.hnsw.search(std::slice::from_ref(query), expanded_k, self.config.ef_search);
        let mut final_results = Vec::with_capacity(results.len());
        for neighbor in results {
            if let Some(id) = self.idx_to_id.get(&neighbor.d_id) {
                final_results.push((id.clone(), query.cosine_similarity(&self.hnsw.get_point_data(&neighbor.p_id).unwrap()[0])));
                if final_results.len() >= top_k { break; }
            }
        }
        Ok(final_results)
    }

    fn search_filtered(&self, query: &H, top_k: usize, filter: &crate::metadata_filter::MetadataFilter, concepts: &HashMap<String, Concept<H>>) -> Result<Vec<(String, f32)>> {
        let expanded_k = top_k * 5 + self.deleted_count.min(top_k * 10);
        let results = self.hnsw.search(std::slice::from_ref(query), expanded_k, self.config.ef_search);
        let mut filtered_results = Vec::new();
        for neighbor in results {
            if let Some(id) = self.idx_to_id.get(&neighbor.d_id) {
                if let Some(concept) = concepts.get(id) {
                    if filter.matches(&concept.metadata) {
                        filtered_results.push((id.clone(), query.cosine_similarity(&concept.vector)));
                        if filtered_results.len() >= top_k { break; }
                    }
                }
            }
        }
        if filtered_results.len() < top_k {
            let mut all_filtered: Vec<(String, f32)> = concepts.iter().filter(|(_, c)| filter.matches(&c.metadata))
                .map(|(id, c)| (id.clone(), query.cosine_similarity(&c.vector))).collect();
            all_filtered.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            all_filtered.truncate(top_k);
            return Ok(all_filtered);
        }
        Ok(filtered_results)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept<H>>) -> Result<()> {
        self.hnsw = Hnsw::new(self.config.m, concepts.len().max(100), 16, self.config.ef_construction, HammingDist(std::marker::PhantomData));
        self.id_to_idx.clear();
        self.idx_to_id.clear();
        self.deleted_count = 0;
        for (id, concept) in concepts { self.insert(id.clone(), &concept.vector)?; }
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        IndexStats { backend: "HNSW".to_string(), count: self.id_to_idx.len(), memory_usage_bytes: self.id_to_idx.len() * (std::mem::size_of::<String>() + std::mem::size_of::<H>() + 32) }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        use std::fs;
        let temp_dir = tempfile::tempdir().map_err(MemoryError::Io)?;
        self.hnsw.file_dump(temp_dir.path(), "index").map_err(|e| MemoryError::database(format!("HNSW dump failed: {}", e)))?;
        let wrapper = HnswPersistenceWrapper {
            id_to_idx: self.id_to_idx.clone(),
            idx_to_id: self.idx_to_id.clone(),
            m: self.config.m,
            ef_construction: self.config.ef_construction,
            ef_search: self.config.ef_search,
            deleted_count: self.deleted_count,
            data: fs::read(temp_dir.path().join("index.hnsw.data")).map_err(MemoryError::Io)?,
            graph: fs::read(temp_dir.path().join("index.hnsw.graph")).map_err(MemoryError::Io)?,
        };
        bincode::serialize(&wrapper).map_err(|e| MemoryError::database(format!("Bincode fail: {}", e)))
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        use std::fs;
        if data.is_empty() { return Ok(()); }
        let wrapper: HnswPersistenceWrapper = bincode::deserialize(data).map_err(|e| MemoryError::database(format!("Bincode fail: {}", e)))?;
        let temp_dir = tempfile::tempdir().map_err(MemoryError::Io)?;
        fs::write(temp_dir.path().join("index.hnsw.data"), &wrapper.data).map_err(MemoryError::Io)?;
        fs::write(temp_dir.path().join("index.hnsw.graph"), &wrapper.graph).map_err(MemoryError::Io)?;
        let loader = HnswIo::new(temp_dir.path(), "index");
        let hnsw = loader.load_hnsw_with_dist::<H, HammingDist<H>>(HammingDist(std::marker::PhantomData))
            .map_err(|e| MemoryError::database(format!("HNSW load failed: {}", e)))?;
        self.hnsw = unsafe { std::mem::transmute(hnsw) };
        self._loader = Some(loader);
        self._temp_dir = Some(temp_dir);
        self.id_to_idx = wrapper.id_to_idx;
        self.idx_to_id = wrapper.idx_to_id;
        self.config.m = wrapper.m;
        self.config.ef_construction = wrapper.ef_construction;
        self.config.ef_search = wrapper.ef_search;
        self.deleted_count = wrapper.deleted_count;
        Ok(())
    }
}
