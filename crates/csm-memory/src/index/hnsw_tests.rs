//! Tests for the HNSW ANN index backend (ADR-0068, ADR-0093).
//!
//! Extracted from `hnsw.rs` to keep that file within the LOC gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::index::AnnIndex;
use crate::singularity::Concept;
use csm_core::error::MemoryError;
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

#[test]
fn new_accepts_default_parameters() {
    let index = HnswIndex::<HVec10240>::new(16, 200, 50).expect("defaults must build");
    assert!(index.core.is_some());
}

#[test]
fn new_rejects_zero_m() {
    let err = HnswIndex::<HVec10240>::new(0, 200, 50).unwrap_err();
    let is_invalid_m = matches!(&err, MemoryError::InvalidInput { field, .. } if field == "m");
    assert!(is_invalid_m, "expected InvalidInput for m, got {err}");
}

#[test]
fn new_rejects_m_above_hnsw_rs_limit() {
    // hnsw_rs hard-exits the process at m > 256; must be caught as an error.
    let err = HnswIndex::<HVec10240>::new(257, 200, 50).unwrap_err();
    let is_invalid_m = matches!(&err, MemoryError::InvalidInput { field, .. } if field == "m");
    assert!(is_invalid_m, "expected InvalidInput for m, got {err}");
}

#[test]
fn new_rejects_zero_ef() {
    for (field, ef_construction, ef_search) in
        [("ef_construction", 0usize, 50usize), ("ef_search", 200, 0)]
    {
        let err = HnswIndex::<HVec10240>::new(16, ef_construction, ef_search).unwrap_err();
        let is_invalid_field = matches!(
            &err,
            MemoryError::InvalidInput { field: f, .. } if f.as_str() == field
        );
        assert!(is_invalid_field, "expected InvalidInput for {field}, got {err}");
    }
}

#[test]
fn invalid_params_fail_closed_for_non_hvec10240() {
    use csm_core::BHVec10240;
    // Even when the graph would fall back to BruteForce, invalid config must error.
    assert!(HnswIndex::<BHVec10240>::new(0, 200, 50).is_err());
}