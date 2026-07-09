#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based security tests for input validation (ADR-0047).
//!
//! Covers: namespace validation, concept ID validation, association strength
//! validation, path traversal protection, metadata size limits, batch size
//! limits, bincode deserialization safety, and null-byte injection using
//! proptest for adversarial input generation.

#![allow(clippy::float_cmp)]

use chaotic_semantic_memory::prelude::*;
use proptest::prelude::*;
use tempfile::NamedTempFile;

// ── Namespace, concept ID, association, path, batch, metadata proptests ─

proptest! {
    #[test]
    fn namespace_with_control_chars_always_rejected(
        ctrl in prop::collection::vec(0u8..=0x1f, 1..5),
        prefix in "[a-zA-Z0-9]{0,10}",
        suffix in "[a-zA-Z0-9]{0,10}",
    ) {
        let mut ns = prefix;
        ns.extend(ctrl.iter().map(|b| char::from(*b)));
        ns.push_str(&suffix);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept(&ns, HVec10240::random()).await
        });
        prop_assert!(result.is_err(), "control chars in namespace should be rejected: {:?}", ns);
    }

    #[test]
    fn oversized_namespace_always_rejected(ns in "[a-zA-Z]{129,500}") {
        // Use set_namespace to test the 128-byte namespace limit directly.
        // inject_concept validates concept IDs (256-byte limit), not namespaces.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.set_namespace(&ns).await
        });
        prop_assert!(result.is_err(), "namespace > 128 bytes should be rejected");
    }

    #[test]
    fn namespace_with_null_byte_rejected(prefix in "[a-zA-Z]{1,10}") {
        let ns = format!("{prefix}\0suffix");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept(&ns, HVec10240::random()).await
        });
        prop_assert!(result.is_err(), "null byte in namespace should be rejected");
    }

    // ── Concept ID validation ────────────────────────────────────────

    #[test]
    fn concept_id_with_control_chars_always_rejected(
        ctrl in prop::collection::vec(0u8..=0x1f, 1..5),
        prefix in "[a-zA-Z0-9]{0,10}",
    ) {
        let mut id = prefix;
        id.extend(ctrl.iter().map(|b| char::from(*b)));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_err(), "control chars in concept ID should be rejected");
    }

    #[test]
    fn oversized_concept_id_always_rejected(id in "[a-zA-Z]{257,1000}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept(&id, HVec10240::random()).await
        });
        prop_assert!(result.is_err(), "concept ID > 256 bytes should be rejected");
    }

    // ── Association strength ─────────────────────────────────────────

    #[test]
    fn out_of_range_association_strength_rejected(strength in -1000.0f32..1000.0f32) {
        prop_assume!(!strength.is_finite() || !(0.0..=1.0).contains(&strength));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("assoc-a", HVec10240::random()).await.unwrap();
            fw.inject_concept("assoc-b", HVec10240::random()).await.unwrap();
            fw.associate("assoc-a", "assoc-b", strength).await
        });
        prop_assert!(result.is_err(), "strength {} should be rejected", strength);
    }

    #[test]
    fn valid_association_strength_accepted(strength in 0.0f32..=1.0f32) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("valid-a", HVec10240::random()).await.unwrap();
            fw.inject_concept("valid-b", HVec10240::random()).await.unwrap();
            fw.associate("valid-a", "valid-b", strength).await
        });
        prop_assert!(result.is_ok(), "strength {} in [0.0,1.0] should be accepted", strength);
    }

    // ── Path traversal ────────────────────────────────────────────────

    #[test]
    fn path_traversal_always_rejected(prefix in "[a-zA-Z]{1,10}", suffix in "[a-zA-Z]{1,10}") {
        let path = format!("/tmp/{prefix}/../../../etc/{suffix}");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("pt", HVec10240::random()).await.unwrap();
            fw.export_json(&path).await
        });
        prop_assert!(result.is_err(), "path traversal should be rejected");
    }

    #[test]
    fn path_exceeding_length_limit_rejected(path in "[a-zA-Z/]{4097,8192}") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("pl", HVec10240::random()).await.unwrap();
            fw.export_json(&path).await
        });
        prop_assert!(result.is_err(), "path > 4096 chars should be rejected");
    }

    #[test]
    fn path_with_null_byte_rejected(segment in "[a-zA-Z]{1,10}") {
        let path = format!("/tmp/\0{segment}");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("nb", HVec10240::random()).await.unwrap();
            fw.export_json(&path).await
        });
        prop_assert!(result.is_err(), "null byte in path should be rejected");
    }

    #[test]
    fn path_with_multiple_traversal_segments_rejected(
        depth in 2usize..6, suffix in "[a-zA-Z]{1,5}",
    ) {
        let traversal = (0..depth).map(|_| "..").collect::<Vec<_>>().join("/");
        let path = format!("/tmp/{traversal}/{suffix}");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            fw.inject_concept("mt", HVec10240::random()).await.unwrap();
            fw.export_json(&path).await
        });
        prop_assert!(result.is_err(), "multi-segment traversal should be rejected");
    }

    // ── Batch size ───────────────────────────────────────────────────

    #[test]
    fn oversized_batch_always_rejected(batch_size in 3usize..100) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().with_max_batch_size(2).build().await.unwrap();
            let concepts: Vec<(String, HVec10240)> = (0..batch_size)
                .map(|i| (format!("b-{i}"), HVec10240::random())).collect();
            fw.inject_concepts(&concepts).await
        });
        prop_assert!(result.is_err(), "batch size {} should be rejected", batch_size);
    }

    // ── Metadata size ────────────────────────────────────────────────

    #[test]
    fn oversized_metadata_always_rejected(
        key in "[a-zA-Z]{1,20}", value_len in 100usize..1000,
    ) {
        let large_value = "x".repeat(value_len);
        let metadata = std::collections::HashMap::from([
            (key, serde_json::json!(large_value)),
        ]);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().with_max_metadata_bytes(50).build().await.unwrap();
            fw.inject_concept_with_metadata("mt", HVec10240::random(), metadata).await
        });
        prop_assert!(result.is_err(), "metadata exceeding limit should be rejected");
    }

    #[test]
    fn multi_key_metadata_exceeding_limit_rejected(
        num_keys in 3usize..8, key_len in 5usize..15, value_len in 50usize..200,
    ) {
        let mut metadata = std::collections::HashMap::new();
        for i in 0..num_keys {
            let key = format!("k{i}_{}", "x".repeat(key_len));
            metadata.insert(key, serde_json::json!("x".repeat(value_len)));
        }
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().with_max_metadata_bytes(100).build().await.unwrap();
            fw.inject_concept_with_metadata("mm", HVec10240::random(), metadata).await
        });
        prop_assert!(result.is_err(), "multi-key metadata exceeding limit should be rejected");
    }

    #[test]
    fn metadata_within_limit_accepted(key in "[a-zA-Z]{3,10}", value in "[a-zA-Z]{1,10}") {
        let metadata = std::collections::HashMap::from([
            (key, serde_json::json!(value)),
        ]);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().with_max_metadata_bytes(10000).build().await.unwrap();
            fw.inject_concept_with_metadata("ok", HVec10240::random(), metadata).await
        });
        prop_assert!(result.is_ok(), "small metadata within limit should be accepted");
    }

    // ── Bincode / JSON deserialization safety ─────────────────────────

    #[test]
    fn import_binary_rejects_random_bytes(data in prop::collection::vec(any::<u8>(), 10..500)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap().to_string();
            tokio::fs::write(&path, &data).await.unwrap();
            fw.import_binary(&path, false).await
        });
        prop_assert!(result.is_err(), "random bytes should be rejected by import_binary");
    }

    #[test]
    fn import_json_rejects_random_bytes(data in prop::collection::vec(any::<u8>(), 10..500)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let fw = ChaoticSemanticFramework::builder()
                .without_persistence().build().await.unwrap();
            let temp = NamedTempFile::new().unwrap();
            let path = temp.path().to_str().unwrap().to_string();
            tokio::fs::write(&path, &data).await.unwrap();
            fw.import_json(&path, false).await
        });
        prop_assert!(result.is_err(), "random bytes should be rejected by import_json");
    }
}

// ── Deterministic edge-case security tests ─────────────────────────────

#[test]
fn namespace_null_byte_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("ns\0injected", HVec10240::random()).await
    });
    assert!(result.is_err(), "null byte in namespace must be rejected");
}

#[test]
fn concept_id_null_byte_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("id\0injected", HVec10240::random()).await
    });
    assert!(result.is_err(), "null byte in concept ID must be rejected");
}

#[test]
fn path_null_byte_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("p", HVec10240::random()).await.unwrap();
        fw.export_json("/tmp/malicious\0.json").await
    });
    assert!(result.is_err(), "null byte in path must be rejected");
}

#[test]
fn path_traversal_parent_dir_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("t", HVec10240::random()).await.unwrap();
        fw.export_json("../../../etc/passwd").await
    });
    assert!(result.is_err(), "'../' path traversal must be rejected");
}

#[test]
fn oversized_binary_import_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap().to_string();
        let f = std::fs::File::create(temp.path()).unwrap();
        f.set_len(101 * 1024 * 1024).unwrap();
        drop(f);
        let result = fw.import_binary(&path, false).await;
        assert!(result.is_err(), "oversized binary import must be rejected");
    });
}

#[test]
fn oversized_json_import_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap().to_string();
        let f = std::fs::File::create(temp.path()).unwrap();
        f.set_len(101 * 1024 * 1024).unwrap();
        drop(f);
        let result = fw.import_json(&path, false).await;
        assert!(result.is_err(), "oversized JSON import must be rejected");
    });
}

#[test]
fn nan_association_strength_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("na", HVec10240::random()).await.unwrap();
        fw.inject_concept("nb", HVec10240::random()).await.unwrap();
        fw.associate("na", "nb", f32::NAN).await
    });
    assert!(result.is_err(), "NaN strength must be rejected");
}

#[test]
fn inf_association_strength_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("ia", HVec10240::random()).await.unwrap();
        fw.inject_concept("ib", HVec10240::random()).await.unwrap();
        fw.associate("ia", "ib", f32::INFINITY).await
    });
    assert!(result.is_err(), "Infinity strength must be rejected");
}

#[test]
fn neg_inf_association_strength_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let fw = ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap();
        fw.inject_concept("na", HVec10240::random()).await.unwrap();
        fw.inject_concept("nb", HVec10240::random()).await.unwrap();
        fw.associate("na", "nb", f32::NEG_INFINITY).await
    });
    assert!(
        result.is_err(),
        "Negative Infinity strength must be rejected"
    );
}
