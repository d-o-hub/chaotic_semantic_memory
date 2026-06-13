#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
//! HNSW ANN index backend (ADR-0068).

// Casts are intentional for similarity math

#[cfg(feature = "ann-hnsw")]
use crate::index::brute_force::BruteForce;
#[cfg(feature = "ann-hnsw")]
use crate::index::{AnnIndex, IndexStats};
#[cfg(feature = "ann-hnsw")]
use crate::singularity::Concept;
#[cfg(feature = "ann-hnsw")]
use csm_core::error::{MemoryError, Result};
#[cfg(feature = "ann-hnsw")]
use csm_core::hyperdim::{HVec10240, Hypervector};
#[cfg(feature = "ann-hnsw")]
use hnsw_rs::prelude::*;
#[cfg(feature = "ann-hnsw")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ann-hnsw")]
use std::any::TypeId;
#[cfg(feature = "ann-hnsw")]
use std::collections::HashMap;

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

/// Non-generic HNSW core — only works with HVec10240.
#[cfg(feature = "ann-hnsw")]
struct HnswCore {
    hnsw: Hnsw<'static, HVec10240, HammingDist>,
    id_to_idx: HashMap<String, usize>,
    idx_to_id: HashMap<usize, String>,
    config: HnswData,
    deleted_count: usize,
    _owner: Option<Box<dyn std::any::Any + Send + Sync>>,
}

#[cfg(feature = "ann-hnsw")]
impl std::fmt::Debug for HnswCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswCore")
            .field("config", &self.config)
            .field("count", &self.id_to_idx.len())
            .finish()
    }
}

/// HNSW ANN index with generic hypervector support.
///
/// For `HVec10240`, uses the actual HNSW graph from `hnsw_rs`.
/// For other hypervector types (e.g., `BHVec10240`), falls back to `BruteForce`.
#[cfg(feature = "ann-hnsw")]
pub struct HnswIndex<H: Hypervector + 'static = HVec10240> {
    core: Option<HnswCore>,
    fallback: BruteForce<H>,
    config: HnswData,
}

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector + 'static> std::fmt::Debug for HnswIndex<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswIndex")
            .field("config", &self.config)
            .field("has_hnsw", &self.core.is_some())
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
impl<H: Hypervector + 'static> HnswIndex<H> {
    pub fn new(m: usize, ef_construction: usize, ef_search: usize) -> Result<Self> {
        let config = HnswData {
            m,
            ef_construction,
            ef_search,
        };

        // Only create the actual HNSW graph when H = HVec10240
        let core = if TypeId::of::<H>() == TypeId::of::<HVec10240>() {
            // #7: Validate m (max_nb_connection). hnsw_rs aborts if > 256.
            if m == 0 || m > 256 {
                return Err(MemoryError::InvalidInput {
                    field: "m".to_string(),
                    reason: "m must be between 1 and 256".to_string(),
                });
            }
            // ADR-0068: Default to 1M elements to support scale goal
            let hnsw = Hnsw::new(m, 1_000_000, 16, ef_construction, HammingDist);
            Some(HnswCore {
                hnsw,
                id_to_idx: HashMap::new(),
                idx_to_id: HashMap::new(),
                config: config.clone(),
                deleted_count: 0,
                _owner: None,
            })
        } else {
            None
        };

        Ok(Self {
            core,
            fallback: BruteForce::new(),
            config,
        })
    }

    /// Check if this instance has a live HNSW graph (i.e., H = HVec10240).
    fn use_hnsw(&self) -> bool {
        self.core.is_some() && TypeId::of::<H>() == TypeId::of::<HVec10240>()
    }

    /// Downcast a generic `&H` to `&HVec10240`.
    ///
    /// # Safety
    /// Caller must verify `TypeId::of::<H>() == TypeId::of::<HVec10240>()` before calling.
    #[inline]
    unsafe fn as_hvec10240(h: &H) -> &HVec10240 {
        debug_assert_eq!(TypeId::of::<H>(), TypeId::of::<HVec10240>());
        unsafe { &*(h as *const H as *const HVec10240) }
    }
}

#[cfg(feature = "ann-hnsw")]
impl<H: Hypervector + 'static> AnnIndex<H> for HnswIndex<H> {
    fn insert(&mut self, id: String, vec: &H) -> Result<()> {
        // Always update fallback
        self.fallback.insert(id.clone(), vec)?;

        let use_hnsw = self.use_hnsw();
        if let Some(core) = &mut self.core {
            if use_hnsw {
                let hvec = unsafe { Self::as_hvec10240(vec) };
                // Handle updates to existing IDs
                if core.id_to_idx.contains_key(&id) {
                    if let Some(idx) = core.id_to_idx.remove(&id) {
                        core.idx_to_id.remove(&idx);
                        core.deleted_count += 1;
                    }
                }
                let idx = core.hnsw.get_nb_point();
                core.hnsw.insert((std::slice::from_ref(hvec), idx));
                core.id_to_idx.insert(id.clone(), idx);
                core.idx_to_id.insert(idx, id);
            }
        }
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        self.fallback.delete(id)?;

        if let Some(core) = &mut self.core {
            // HnswIndex::delete only removes mappings
            if let Some(idx) = core.id_to_idx.remove(id) {
                core.idx_to_id.remove(&idx);
                core.deleted_count += 1;
            }
        }
        Ok(())
    }

    fn search(&self, query: &H, top_k: usize) -> Result<Vec<(String, f32)>> {
        if let Some(core) = &self.core {
            if self.use_hnsw() {
                let hvec = unsafe { Self::as_hvec10240(query) };
                // Increase search budget to account for deleted nodes
                let expanded_k = top_k + core.deleted_count.min(top_k * 10);
                let results = core.hnsw.search(
                    std::slice::from_ref(hvec),
                    expanded_k,
                    core.config.ef_search,
                );

                let mut final_results = Vec::with_capacity(results.len());
                for neighbor in results {
                    if let Some(id) = core.idx_to_id.get(&neighbor.d_id) {
                        let similarity = 1.0 - (neighbor.distance / 5120.0);
                        final_results.push((id.clone(), similarity));
                        if final_results.len() >= top_k {
                            break;
                        }
                    }
                }
                return Ok(final_results);
            }
        }

        self.fallback.search(query, top_k)
    }

    fn search_filtered(
        &self,
        query: &H,
        top_k: usize,
        filter: &crate::metadata_filter::MetadataFilter,
        concepts: &HashMap<String, Concept<H>>,
    ) -> Result<Vec<(String, f32)>> {
        if let Some(core) = &self.core {
            if self.use_hnsw() {
                let hvec = unsafe { Self::as_hvec10240(query) };
                let expanded_k = top_k * 5 + core.deleted_count.min(top_k * 10);
                let results = core.hnsw.search(
                    std::slice::from_ref(hvec),
                    expanded_k,
                    core.config.ef_search,
                );

                let mut filtered_results = Vec::new();
                for neighbor in results {
                    if let Some(id) = core.idx_to_id.get(&neighbor.d_id) {
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

                if filtered_results.len() < top_k {
                    let mut all_filtered: Vec<(String, f32)> = concepts
                        .iter()
                        .filter(|(_, c)| filter.matches(&c.metadata))
                        .map(|(id, c)| {
                            let dist =
                                hvec.hamming_distance(unsafe { Self::as_hvec10240(&c.vector) });
                            (id.clone(), 1.0 - (dist as f32 / 5120.0))
                        })
                        .collect();

                    all_filtered.sort_unstable_by(|a, b| {
                        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    all_filtered.truncate(top_k);
                    return Ok(all_filtered);
                }

                return Ok(filtered_results);
            }
        }

        self.fallback
            .search_filtered(query, top_k, filter, concepts)
    }

    fn rebuild(&mut self, concepts: &HashMap<String, Concept<H>>) -> Result<()> {
        self.fallback.rebuild(concepts)?;

        let use_hnsw = self.use_hnsw();
        if let Some(core) = &mut self.core {
            if use_hnsw {
                core.hnsw = Hnsw::new(
                    core.config.m,
                    concepts.len().max(100),
                    16,
                    core.config.ef_construction,
                    HammingDist,
                );
                core.id_to_idx.clear();
                core.idx_to_id.clear();
                core._owner = None;
                core.deleted_count = 0;

                for (id, concept) in concepts {
                    let hvec = unsafe { Self::as_hvec10240(&concept.vector) };
                    let idx = core.hnsw.get_nb_point();
                    core.hnsw.insert((std::slice::from_ref(hvec), idx));
                    core.id_to_idx.insert(id.clone(), idx);
                    core.idx_to_id.insert(idx, id.clone());
                }
            }
        }
        Ok(())
    }

    fn stats(&self) -> IndexStats {
        if let Some(core) = &self.core {
            if self.use_hnsw() {
                return IndexStats {
                    backend: "HNSW".to_string(),
                    count: core.id_to_idx.len(),
                    memory_usage_bytes: core.id_to_idx.len()
                        * (std::mem::size_of::<String>() + std::mem::size_of::<HVec10240>() + 32),
                };
            }
        }
        self.fallback.stats()
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        if self.use_hnsw() {
            if let Some(core) = &self.core {
                use std::fs;

                let nonce = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let temp_dir = std::env::temp_dir().join(format!("csm_hnsw_{nonce}"));
                fs::create_dir_all(&temp_dir).map_err(MemoryError::Io)?;

                core.hnsw
                    .file_dump(&temp_dir, "index")
                    .map_err(|e| MemoryError::database(format!("HNSW dump failed: {e}")))?;

                let data_path = temp_dir.join("index.hnsw.data");
                let graph_path = temp_dir.join("index.hnsw.graph");

                let data_bytes = fs::read(data_path).map_err(MemoryError::Io)?;
                let graph_bytes = fs::read(graph_path).map_err(MemoryError::Io)?;

                let wrapper = HnswPersistenceWrapper {
                    id_to_idx: core.id_to_idx.clone(),
                    idx_to_id: core.idx_to_id.clone(),
                    m: core.config.m,
                    ef_construction: core.config.ef_construction,
                    ef_search: core.config.ef_search,
                    deleted_count: core.deleted_count,
                    data: data_bytes,
                    graph: graph_bytes,
                };

                let payload = bincode::serialize(&wrapper)
                    .map_err(|e| MemoryError::database(format!("Bincode fail: {e}")))?;

                let _ = fs::remove_dir_all(temp_dir);
                return Ok(payload);
            }
        }
        self.fallback.serialize()
    }

    fn deserialize(&mut self, data: &[u8]) -> Result<()> {
        let use_hnsw = self.use_hnsw();
        if let Some(core) = &mut self.core {
            if use_hnsw {
                use std::fs;

                if data.is_empty() {
                    return Ok(());
                }

                let wrapper: HnswPersistenceWrapper = bincode::deserialize(data)
                    .map_err(|e| MemoryError::database(format!("Bincode deserialize fail: {e}")))?;

                let temp_dir = tempfile::tempdir().map_err(MemoryError::Io)?;
                let path = temp_dir.path();

                fs::write(path.join("index.hnsw.data"), &wrapper.data).map_err(MemoryError::Io)?;
                fs::write(path.join("index.hnsw.graph"), &wrapper.graph)
                    .map_err(MemoryError::Io)?;

                let loader = HnswIo::new(path, "index");
                let hnsw = loader
                    .load_hnsw_with_dist::<HVec10240, HammingDist>(HammingDist)
                    .map_err(|e| MemoryError::database(format!("HNSW load failed: {e}")))?;

                // SAFETY: The Hnsw instance returned by load_hnsw_with_dist may contain
                // references to the loader or memory-mapped files. We transmute it to
                // 'static to store it in our struct, but we keep the source alive by
                // storing them in the _owner field.
                let static_hnsw: Hnsw<'static, HVec10240, HammingDist> =
                    unsafe { std::mem::transmute(hnsw) };

                core.hnsw = static_hnsw;
                core.id_to_idx = wrapper.id_to_idx;
                core.idx_to_id = wrapper.idx_to_id;
                core.config.m = wrapper.m;
                core.config.ef_construction = wrapper.ef_construction;
                core.config.ef_search = wrapper.ef_search;
                core.deleted_count = wrapper.deleted_count;

                // Keep the loader and temp_dir alive to prevent UAF
                core._owner = Some(Box::new((loader, temp_dir)));
                return Ok(());
            }
        }
        self.fallback.deserialize(data)
    }
}

#[cfg(all(test, feature = "ann-hnsw"))]
mod tests {
    use super::*;
    use crate::index::AnnIndex;
    use crate::singularity::Concept;
    use csm_core::hyperdim::HVec10240;
    use std::collections::HashMap;

    // Skip under Miri: hnsw_rs 0.3.4 creates unaligned &[HVec10240] references
    // during deserialization (hnswio.rs:1163 from_raw_parts cast). Third-party bug.
    #[cfg(not(miri))]
    #[test]
    fn test_persistence_roundtrip_miri() -> Result<()> {
        let mut index = HnswIndex::<HVec10240>::new(16, 100, 10)?;
        let id = "test".to_string();
        let vec = HVec10240::random();
        index.insert(id.clone(), &vec)?;

        let serialized = index.serialize()?;
        let mut new_index = HnswIndex::<HVec10240>::new(16, 100, 10)?;
        new_index.deserialize(&serialized)?;

        let results = new_index.search(&vec, 1)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        Ok(())
    }

    #[test]
    fn test_rebuild_resets_owner() -> Result<()> {
        let mut index = HnswIndex::<HVec10240>::new(16, 100, 10)?;
        let id = "test".to_string();
        let vec = HVec10240::random();
        index.insert(id.clone(), &vec)?;

        // Simulate a load that sets _owner
        let serialized = index.serialize()?;
        index.deserialize(&serialized)?;
        assert!(index.core.as_ref().is_some_and(|c| c._owner.is_some()));

        let mut concepts = HashMap::new();
        concepts.insert(
            id.clone(),
            Concept {
                id,
                vector: vec,
                ..Default::default()
            },
        );

        index.rebuild(&concepts)?;
        assert!(index.core.as_ref().is_some_and(|c| c._owner.is_none()));
        Ok(())
    }

    #[test]
    fn binary_singularity_type_alias_works() {
        let _bs: crate::singularity::BinarySingularity =
            crate::singularity::Singularity::new(crate::singularity::SingularityConfig::default());
    }

    #[test]
    fn hnsw_index_bruteforce_fallback_for_binary_vectors() {
        use csm_core::BHVec10240;

        // When H != HVec10240, HnswIndex should fall back to BruteForce
        let mut index = HnswIndex::<BHVec10240>::new(16, 100, 10).unwrap();
        assert!(!index.use_hnsw(), "BHVec10240 should not use HNSW graph");

        let vec = BHVec10240::random();
        index.insert("bin-1".to_string(), &vec).unwrap();

        let results = index.search(&vec, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "bin-1");

        let stats = index.stats();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.backend, "BruteForce");
    }
}
