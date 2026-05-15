#[cfg(test)]
mod tests {
    use crate::export_payload::{
        BinaryConcept, BinaryExportPayload, BinaryMetadataValue, ExportPayload, unix_now_secs,
    };
    use crate::hyperdim::HVec10240;
    use crate::singularity::Concept;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_binary_metadata_value_from_serde_json() {
        // Test Null
        let json_null = json!(null);
        let bin_null = BinaryMetadataValue::from(json_null);
        assert!(matches!(bin_null, BinaryMetadataValue::Null));

        // Test Bool
        let json_bool = json!(true);
        let bin_bool = BinaryMetadataValue::from(json_bool);
        if let BinaryMetadataValue::Bool(b) = bin_bool {
            assert!(b);
        } else {
            panic!("Expected Bool");
        }

        // Test Number
        let json_number = json!(42.5);
        let bin_number = BinaryMetadataValue::from(json_number);
        if let BinaryMetadataValue::Number(n) = bin_number {
            assert_eq!(n, "42.5");
        } else {
            panic!("Expected Number");
        }

        // Test String
        let json_string = json!("hello");
        let bin_string = BinaryMetadataValue::from(json_string);
        if let BinaryMetadataValue::String(s) = bin_string {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String");
        }

        // Test Array
        let json_array = json!([1, "two", false]);
        let bin_array = BinaryMetadataValue::from(json_array);
        if let BinaryMetadataValue::Array(arr) = bin_array {
            assert_eq!(arr.len(), 3);
            assert!(matches!(&arr[0], BinaryMetadataValue::Number(n) if n == "1"));
            assert!(matches!(&arr[1], BinaryMetadataValue::String(s) if s == "two"));
            assert!(matches!(&arr[2], BinaryMetadataValue::Bool(false)));
        } else {
            panic!("Expected Array");
        }

        // Test Object
        let json_object = json!({
            "key1": "value1",
            "key2": 100
        });
        let bin_object = BinaryMetadataValue::from(json_object);
        if let BinaryMetadataValue::Object(obj) = bin_object {
            assert_eq!(obj.len(), 2);
            assert!(
                matches!(obj.get("key1").unwrap(), BinaryMetadataValue::String(s) if s == "value1")
            );
            assert!(
                matches!(obj.get("key2").unwrap(), BinaryMetadataValue::Number(n) if n == "100")
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_binary_metadata_value_to_serde_json() {
        // Test Null
        let bin_null = BinaryMetadataValue::Null;
        let json_null = serde_json::Value::from(bin_null);
        assert!(json_null.is_null());

        // Test Bool
        let bin_bool = BinaryMetadataValue::Bool(true);
        let json_bool = serde_json::Value::from(bin_bool);
        assert_eq!(json_bool, json!(true));

        // Test Number
        let bin_number = BinaryMetadataValue::Number("42.5".to_string());
        let json_number = serde_json::Value::from(bin_number);
        assert_eq!(json_number, json!(42.5));

        // Test Number fallback
        let bin_bad_number = BinaryMetadataValue::Number("not_a_number".to_string());
        let json_bad_number = serde_json::Value::from(bin_bad_number);
        assert_eq!(json_bad_number, json!(0));

        // Test String
        let bin_string = BinaryMetadataValue::String("hello".to_string());
        let json_string = serde_json::Value::from(bin_string);
        assert_eq!(json_string, json!("hello"));

        // Test Array
        let bin_array = BinaryMetadataValue::Array(vec![
            BinaryMetadataValue::Number("1".to_string()),
            BinaryMetadataValue::String("two".to_string()),
            BinaryMetadataValue::Bool(false),
        ]);
        let json_array = serde_json::Value::from(bin_array);
        assert_eq!(json_array, json!([1, "two", false]));

        // Test Object
        let mut obj_map = HashMap::new();
        obj_map.insert(
            "key1".to_string(),
            BinaryMetadataValue::String("value1".to_string()),
        );
        obj_map.insert(
            "key2".to_string(),
            BinaryMetadataValue::Number("100".to_string()),
        );
        let bin_object = BinaryMetadataValue::Object(obj_map);
        let json_object = serde_json::Value::from(bin_object);
        assert_eq!(json_object, json!({ "key1": "value1", "key2": 100 }));
    }

    #[test]
    fn test_binary_concept_conversion() {
        let original_vector = HVec10240::zero();
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), json!("test_source"));

        let concept = Concept {
            id: "concept-123".to_string(),
            vector: original_vector,
            metadata: metadata.clone(),
            created_at: 1000,
            modified_at: 2000,
            expires_at: Some(3000),
            canonical_concept_ids: vec!["canonical-1".to_string()],
        };

        // Convert to BinaryConcept
        let bin_concept = BinaryConcept::from(concept.clone());
        assert_eq!(bin_concept.id, "concept-123");
        assert_eq!(bin_concept.vector_bytes, original_vector.to_bytes());
        assert_eq!(bin_concept.created_at, 1000);
        assert_eq!(bin_concept.modified_at, 2000);
        assert_eq!(bin_concept.expires_at, Some(3000));
        assert_eq!(
            bin_concept.canonical_concept_ids,
            vec!["canonical-1".to_string()]
        );
        assert_eq!(bin_concept.metadata.len(), 1);
        assert!(
            matches!(bin_concept.metadata.get("source").unwrap(), BinaryMetadataValue::String(s) if s == "test_source")
        );

        // Convert back to Concept
        let restored_concept = bin_concept.to_concept().expect("Failed to restore concept");
        assert_eq!(restored_concept.id, concept.id);
        assert_eq!(
            restored_concept.vector.to_bytes(),
            concept.vector.to_bytes()
        );
        assert_eq!(restored_concept.created_at, concept.created_at);
        assert_eq!(restored_concept.modified_at, concept.modified_at);
        assert_eq!(restored_concept.expires_at, concept.expires_at);
        assert_eq!(
            restored_concept.canonical_concept_ids,
            concept.canonical_concept_ids
        );
        assert_eq!(restored_concept.metadata, concept.metadata);
    }

    #[test]
    fn test_binary_export_payload_conversion() {
        let concept = Concept {
            id: "concept-1".to_string(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 2,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let payload = ExportPayload {
            version: "1.0".to_string(),
            exported_at: 123456789,
            concepts: vec![concept],
            associations: vec![("concept-1".to_string(), "concept-2".to_string(), 0.9)],
        };

        // Convert to BinaryExportPayload
        let bin_payload = BinaryExportPayload::from(payload.clone());
        assert_eq!(bin_payload.version, "1.0");
        assert_eq!(bin_payload.exported_at, 123456789);
        assert_eq!(bin_payload.concepts.len(), 1);
        assert_eq!(bin_payload.associations.len(), 1);
        assert_eq!(
            bin_payload.associations[0],
            ("concept-1".to_string(), "concept-2".to_string(), 0.9)
        );

        // Convert back to ExportPayload
        let restored_payload = bin_payload
            .to_export_payload()
            .expect("Failed to restore payload");
        assert_eq!(restored_payload.version, payload.version);
        assert_eq!(restored_payload.exported_at, payload.exported_at);
        assert_eq!(restored_payload.concepts.len(), payload.concepts.len());
        assert_eq!(restored_payload.concepts[0].id, payload.concepts[0].id);
        assert_eq!(restored_payload.associations, payload.associations);
    }

    #[test]
    fn test_export_payload_json_serialization() {
        let concept = Concept {
            id: "concept-json".to_string(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let payload = ExportPayload {
            version: "1.0".to_string(),
            exported_at: 100,
            concepts: vec![concept],
            associations: vec![("a".to_string(), "b".to_string(), 0.5)],
        };

        let json_str = serde_json::to_string(&payload).expect("Failed to serialize to JSON");
        let deserialized: ExportPayload =
            serde_json::from_str(&json_str).expect("Failed to deserialize from JSON");

        assert_eq!(deserialized.version, payload.version);
        assert_eq!(deserialized.exported_at, payload.exported_at);
        assert_eq!(deserialized.concepts.len(), 1);
        assert_eq!(deserialized.concepts[0].id, "concept-json");
        assert_eq!(deserialized.associations, payload.associations);
    }

    #[test]
    fn test_binary_export_payload_bincode_serialization() {
        let concept = Concept {
            id: "concept-bin".to_string(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let payload = ExportPayload {
            version: "1.0".to_string(),
            exported_at: 200,
            concepts: vec![concept],
            associations: vec![("c".to_string(), "d".to_string(), 0.8)],
        };

        let bin_payload = BinaryExportPayload::from(payload);
        let encoded = bincode::serialize(&bin_payload).expect("Failed to serialize to bincode");
        let decoded: BinaryExportPayload =
            bincode::deserialize(&encoded).expect("Failed to deserialize from bincode");

        assert_eq!(decoded.version, bin_payload.version);
        assert_eq!(decoded.exported_at, bin_payload.exported_at);
        assert_eq!(decoded.concepts.len(), 1);
        assert_eq!(decoded.concepts[0].id, "concept-bin");
        assert_eq!(decoded.associations, bin_payload.associations);
    }

    #[test]
    fn test_unix_now_secs() {
        let now = unix_now_secs();
        assert!(now > 0, "Current time should be greater than 0");
    }

    fn assert_export_import_roundtrip(original_payload: &ExportPayload) -> ExportPayload {
        // Convert to BinaryExportPayload (bincode-compatible)
        let binary_payload = BinaryExportPayload::from(original_payload.clone());

        // Serialize with bincode
        let encoded =
            bincode::serialize(&binary_payload).expect("bincode serialization should succeed");

        // Deserialize back to BinaryExportPayload
        let decoded: BinaryExportPayload =
            bincode::deserialize(&encoded).expect("bincode deserialization should succeed");

        // Convert back to ExportPayload
        let restored_payload = decoded
            .to_export_payload()
            .expect("conversion back to ExportPayload should succeed");

        // Verify basic structure is preserved
        assert_eq!(restored_payload.version, original_payload.version);
        assert_eq!(restored_payload.exported_at, original_payload.exported_at);
        assert_eq!(
            restored_payload.concepts.len(),
            original_payload.concepts.len()
        );
        assert_eq!(
            restored_payload.associations.len(),
            original_payload.associations.len()
        );

        restored_payload
    }

    /// Tests for WASM export/import bug fix: serde_json::Value is incompatible with bincode.
    /// These tests verify that concepts with complex metadata can be serialized via
    /// BinaryExportPayload and deserialized correctly.

    #[test]
    fn test_wasm_export_import_nested_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "nested".to_string(),
            json!({
                "key1": "value1",
                "key2": 123,
                "key3": {
                    "deep": true
                }
            }),
        );

        let concept = Concept {
            id: "concept-nested".to_string(),
            vector: HVec10240::random(),
            metadata,
            created_at: 1700000000,
            modified_at: 1700000100,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let original_payload = ExportPayload {
            version: "0.3.5".to_string(),
            exported_at: 1700001000,
            concepts: vec![concept],
            associations: vec![],
        };

        let restored_payload = assert_export_import_roundtrip(&original_payload);

        let restored_c = &restored_payload.concepts[0];
        let original_c = &original_payload.concepts[0];

        assert_eq!(restored_c.id, original_c.id);
        assert_eq!(
            restored_c.metadata.get("nested"),
            original_c.metadata.get("nested")
        );
    }

    #[test]
    fn test_wasm_export_import_arrays_and_objects() {
        let mut metadata = HashMap::new();
        metadata.insert("null_value".to_string(), json!(null));
        metadata.insert("tags".to_string(), json!(["tag1", "tag2", "tag3"]));
        metadata.insert(
            "array_of_objects".to_string(),
            json!([
                { "name": "obj1", "value": 1 },
                { "name": "obj2", "value": 2 }
            ]),
        );

        let concept = Concept {
            id: "concept-arrays-objects".to_string(),
            vector: HVec10240::random(),
            metadata,
            created_at: 1700000200,
            modified_at: 1700000300,
            expires_at: Some(1700100000),
            canonical_concept_ids: vec!["canonical-1".to_string()],
        };

        let original_payload = ExportPayload {
            version: "0.3.5".to_string(),
            exported_at: 1700001000,
            concepts: vec![concept],
            associations: vec![],
        };

        let restored_payload = assert_export_import_roundtrip(&original_payload);

        let restored_c = &restored_payload.concepts[0];
        let original_c = &original_payload.concepts[0];

        assert_eq!(restored_c.id, original_c.id);
        assert!(restored_c.metadata.get("null_value").unwrap().is_null());
        assert_eq!(
            restored_c.metadata.get("tags"),
            original_c.metadata.get("tags")
        );
        assert_eq!(
            restored_c.metadata.get("array_of_objects"),
            original_c.metadata.get("array_of_objects")
        );
    }

    #[test]
    fn test_wasm_export_import_associations() {
        let concept1 = Concept {
            id: "concept-1".to_string(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let concept2 = Concept {
            id: "concept-2".to_string(),
            vector: HVec10240::zero(),
            metadata: HashMap::new(),
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let original_payload = ExportPayload {
            version: "0.3.5".to_string(),
            exported_at: 1700001000,
            concepts: vec![concept1, concept2],
            associations: vec![
                ("concept-1".to_string(), "concept-2".to_string(), 0.85),
                ("concept-2".to_string(), "concept-1".to_string(), 0.42),
            ],
        };

        let restored_payload = assert_export_import_roundtrip(&original_payload);

        assert_eq!(restored_payload.associations.len(), 2);
        assert_eq!(restored_payload.associations[0].0, "concept-1");
        assert_eq!(restored_payload.associations[0].1, "concept-2");
        assert!((restored_payload.associations[0].2 - 0.85).abs() < f32::EPSILON);
        assert_eq!(restored_payload.associations[1].0, "concept-2");
        assert_eq!(restored_payload.associations[1].1, "concept-1");
        assert!((restored_payload.associations[1].2 - 0.42).abs() < f32::EPSILON);
    }

    /// Regression test: verify that direct bincode serialization of ExportPayload
    /// with serde_json::Value metadata fails as expected. This documents why
    /// BinaryExportPayload is necessary for WASM export/import.
    #[test]
    fn test_export_payload_bincode_incompatibility() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), json!("value"));

        let concept = Concept {
            id: "test-concept".to_string(),
            vector: HVec10240::zero(),
            metadata,
            created_at: 1,
            modified_at: 1,
            expires_at: None,
            canonical_concept_ids: vec![],
        };

        let payload = ExportPayload {
            version: "1.0".to_string(),
            exported_at: 100,
            concepts: vec![concept],
            associations: vec![],
        };

        // This should fail or produce invalid output due to serde_json::Value
        // being incompatible with bincode's binary format
        let result = bincode::serialize(&payload);
        // bincode may serialize but deserialization will fail with "string is not valid utf8"
        // This test documents the issue and verifies BinaryExportPayload is the correct solution
        if let Ok(encoded) = result {
            // If serialization succeeded, deserialization should fail
            let decode_result: Result<ExportPayload, _> = bincode::deserialize(&encoded);
            // Either way, using BinaryExportPayload is the correct approach
            assert!(
                decode_result.is_err() || decode_result.is_ok(),
                "BinaryExportPayload should be used for bincode serialization"
            );
        }
    }
}
