# ADR-0058: Fix Import/Export Serialization for JSON and Binary Formats

## Status

Accepted

## Context

The import/export functionality for the CSM CLI was broken:

1. **JSON export/import failed** with error: `invalid type: sequence, expected a byte array of length 1280`
   - The `HVec10240` struct was using `serialize_bytes` which produces an array of numbers in JSON
   - When deserializing, it expected `deserialize_bytes` but received a sequence

2. **Binary export/import failed** with error: `Bincode does not support the serde::Deserializer::deserialize_any method`
   - The `HVec10240` deserialization used `is_human_readable()` to choose between formats
   - This internally uses `deserialize_any` which bincode doesn't support
   - Additionally, `serde_json::Value` in metadata also uses `deserialize_any`

These issues prevented users from backing up and restoring their memory databases.

## Decision

Implement comprehensive serialization fixes with separate code paths for JSON and binary formats.

### 1. HVec10240 JSON Serialization (Base64)

Change `HVec10240` serialization to use base64 encoding for human-readable formats:

```rust
impl Serialize for HVec10240 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Use base64 for JSON
            let bytes = self.to_bytes();
            let b64 = STANDARD.encode(&bytes);
            serializer.serialize_str(&b64)
        } else {
            // Use raw bytes for binary
            serializer.serialize_bytes(&bytes)
        }
    }
}
```

### 2. Binary-Compatible Export Payload

Create a separate `BinaryExportPayload` struct that avoids `serde_json::Value`:

```rust
/// Binary-compatible metadata value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum BinaryMetadataValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<BinaryMetadataValue>),
    Object(HashMap<String, BinaryMetadataValue>),
}

/// Concept representation for binary export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BinaryConcept {
    pub(crate) id: String,
    pub(crate) vector_bytes: Vec<u8>,  // Raw bytes instead of HVec10240
    pub(crate) metadata: HashMap<String, BinaryMetadataValue>,
    pub(crate) created_at: u64,
    pub(crate) modified_at: u64,
}
```

### 3. Consistent Bincode Options

Use the same bincode options for both serialization and deserialization:

```rust
let options = bincode::DefaultOptions::new().with_limit(MAX_IMPORT_SIZE);
let data = options.serialize(&payload)?;  // Export
let payload: BinaryExportPayload = options.deserialize(&bytes)?;  // Import
```

## Consequences

### Positive

1. **JSON roundtrip works**: Export to JSON and import back successfully
2. **Binary roundtrip works**: Export to binary format and import back successfully
3. **Data integrity preserved**: All concepts, associations, and metadata are preserved
4. **Human-readable JSON**: Vectors are now base64 strings instead of huge arrays
5. **Efficient binary format**: Compact bincode serialization for large datasets

### Negative

1. **Breaking change**: Old binary exports are no longer compatible (different format)
2. **Additional complexity**: Two separate payload structs to maintain
3. **Memory overhead**: BinaryMetadataValue is less efficient than serde_json::Value

### Neutral

1. **JSON exports are larger**: Base64 encoding adds ~33% overhead vs raw bytes
2. **WASM builds require annotations**: `#[allow(dead_code)]` needed for unused structs

## Migration Guide

### For Users with Old Exports

Old JSON exports with array-style vectors will still work - the deserializer accepts both formats.

Old binary exports will fail to import. Users should:
1. Re-export using the new version
2. Or manually convert if needed

### For Developers

When adding new metadata types, ensure they can be converted to/from `BinaryMetadataValue`.

## Validation

```bash
# Run the verification script
./scripts/verify-memory-roundtrip.sh

# Expected output:
# Tests Passed: 5
# Tests Failed: 0
```

## References

- PR: https://github.com/d-o-hub/chaotic_semantic_memory/pull/32
- Skill: `.agents/skills/turso-memory-verification/SKILL.md`
- Script: `scripts/verify-memory-roundtrip.sh`

## Decision Record

**Date:** 2026-03-16
**Decision:** Implement base64 for JSON and BinaryMetadataValue for bincode
**Status:** Accepted
**Owner:** opencode agent
**Stakeholders:** All CSM users

**Rationale:** The previous serialization was fundamentally incompatible with both JSON and bincode requirements. The new approach provides working roundtrip serialization for both formats.
