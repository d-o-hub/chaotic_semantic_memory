//! Roundtrip tests for LshIndex serialize/deserialize (AnnIndex trait).
//!
//! Kills mutations that replace serialize/deserialize bodies with dummy
//! Ok(vec![]), Ok(vec![0]), Ok(vec![1]), or Ok(()).
//!
//! If serialize returns an empty or dummy vector, the deserialized index
//! will be empty and searches will fail to find inserted concepts.
//! If deserialize is a no-op, the target index remains empty.

#![cfg(feature = "ann-lsh")]

use chaotic_semantic_memory::index::{IndexBackend, create_index};
use csm_core::hyperdim::HVec10240;

fn make_vec(set_bits: &[usize]) -> HVec10240 {
    let mut v = HVec10240::zero();
    for &b in set_bits {
        v.set_bit(b);
    }
    v
}

#[test]
fn serialize_produces_nonempty_payload() {
    let backend = IndexBackend::Lsh {
        num_tables: 4,
        hash_bits: 16,
    };
    let mut index = create_index(&backend).unwrap();
    index.insert("a".into(), &make_vec(&[0, 1, 2])).unwrap();
    index
        .insert("b".into(), &make_vec(&[100, 200, 300]))
        .unwrap();

    let data = index.serialize().expect("serialize must succeed");

    // Ok(vec![]) mutant — serialize must return non-empty bytes.
    assert!(!data.is_empty(), "serialize returned empty bytes");
    // Ok(vec![0]) and Ok(vec![1]) mutants — must be more than 1 byte.
    assert!(
        data.len() > 1,
        "serialize returned only {} byte(s)",
        data.len()
    );
    // A bincode-serialised HashMap with 2 concepts will be well over 10 bytes.
    assert!(
        data.len() > 10,
        "serialized {} bytes, expected >10",
        data.len()
    );
}

#[test]
fn deserialize_restores_index_state() {
    let backend = IndexBackend::Lsh {
        num_tables: 4,
        hash_bits: 16,
    };
    let mut src = create_index(&backend).unwrap();
    src.insert("x".into(), &make_vec(&[0, 1, 2])).unwrap();
    src.insert("y".into(), &make_vec(&[100, 200, 300])).unwrap();
    src.insert("z".into(), &make_vec(&[500, 600])).unwrap();

    let serialized = src.serialize().unwrap();

    // Ok(()) deserialize mutant — dst must not remain empty.
    let mut dst = create_index(&backend).unwrap();
    dst.deserialize(&serialized)
        .expect("deserialize must succeed");

    let stats_dst = dst.stats();
    assert_eq!(
        stats_dst.count, 3,
        "expected 3 concepts after deserialization"
    );
}

#[test]
fn roundtrip_search_results_match() {
    let backend = IndexBackend::Lsh {
        num_tables: 4,
        hash_bits: 16,
    };
    let mut src = create_index(&backend).unwrap();
    src.insert("alpha".into(), &make_vec(&[0, 1, 2, 3]))
        .unwrap();
    src.insert("beta".into(), &make_vec(&[100, 101, 102]))
        .unwrap();
    src.insert("gamma".into(), &make_vec(&[500, 501])).unwrap();

    let serialized = src.serialize().unwrap();
    let mut dst = create_index(&backend).unwrap();
    dst.deserialize(&serialized).unwrap();

    let query = make_vec(&[0, 1, 2, 3]);
    let mut src_results: Vec<_> = src.search(&query, 3).unwrap();
    let mut dst_results: Vec<_> = dst.search(&query, 3).unwrap();
    src_results.sort_by(|a, b| a.0.cmp(&b.0));
    dst_results.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(src_results.len(), dst_results.len());
    for (s, d) in src_results.iter().zip(dst_results.iter()) {
        assert_eq!(s.0, d.0, "concept IDs must match");
        assert!((s.1 - d.1).abs() < 1e-6, "scores differ for '{}'", s.0);
    }
}

#[test]
fn deserialize_replaces_old_state() {
    let backend = IndexBackend::Lsh {
        num_tables: 4,
        hash_bits: 16,
    };
    // Start with 3 concepts, then deserialize 1 concept over it.
    let mut src1 = create_index(&backend).unwrap();
    src1.insert("only-one".into(), &make_vec(&[10, 20]))
        .unwrap();
    let data1 = src1.serialize().unwrap();

    let mut src2 = create_index(&backend).unwrap();
    src2.insert("a".into(), &make_vec(&[0])).unwrap();
    src2.insert("b".into(), &make_vec(&[1])).unwrap();
    src2.insert("c".into(), &make_vec(&[2])).unwrap();

    let mut target = create_index(&backend).unwrap();
    target.deserialize(&src2.serialize().unwrap()).unwrap();
    assert_eq!(target.stats().count, 3);

    target.deserialize(&data1).unwrap();
    assert_eq!(target.stats().count, 1, "must replace old state");

    let results = target.search(&make_vec(&[10, 20]), 1).unwrap();
    assert_eq!(results[0].0, "only-one");
}
