---
name: turso-memory-verification
description: "Verify memory persistence with Turso/libSQL databases before releases. Use before any release to ensure import/export functionality works correctly, data roundtrips successfully, and no serialization regressions."
---

# Skill: turso-memory-verification

Verify memory persistence with Turso/libSQL databases before releases.

## When to Use

Use this skill before any release to ensure:
- Import/export functionality works correctly
- Data roundtrips successfully between databases
- No serialization regressions

## Workflow

1. **Local SQLite Test**
   ```bash
   # Create test database
   db1=".tmp/turso-test-$(date +%s)-1.db"
   db2=".tmp/turso-test-$(date +%s)-2.db"
   
   # Inject test concepts
   csm --database "$db1" inject "test::concept::1" -m '{"context":"test"}'
   csm --database "$db1" inject "test::concept::2" -m '{"context":"test2"}'
   csm --database "$db1" associate "test::concept::1" "test::concept::2" -s 0.85
   
   # Test JSON roundtrip
   csm --database "$db1" export -o export.json
   csm --database "$db2" import export.json
   
   # Verify
   csm --database "$db2" probe "test::concept::1"
   ```

2. **Binary Format Test**
   ```bash
   # Export as binary
   csm --database "$db1" export --format binary -o export.bin
   
   # Import to new database
   csm --database "$db3" import --format binary export.bin
   
   # Verify data integrity
   csm --database "$db3" export --output-format json | jq '.concepts | length'
   ```

3. **Automated Validation Script**
   ```bash
   source scripts/verify-memory-roundtrip.sh
   ```

## Verification Checklist

- [ ] JSON export creates valid file
- [ ] JSON import loads all concepts
- [ ] Binary export creates valid file
- [ ] Binary import loads all concepts
- [ ] Associations are preserved
- [ ] Metadata is preserved
- [ ] Vector similarity works after import

## Common Issues

### Import Fails with "deserialize_any"
**Cause**: `serde_json::Value` in metadata is incompatible with bincode.
**Fix**: Use `BinaryMetadataValue` enum for binary serialization.

### JSON Shows Array Instead of Base64
**Cause**: HVec10240 using `serialize_bytes` for JSON.
**Fix**: Use base64 encoding for human-readable formats.

### File Size Mismatch
**Cause**: Inconsistent bincode options between export/import.
**Fix**: Use `bincode::DefaultOptions::new()` for both operations.

## References

- `src/export_payload.rs` - Binary-compatible payload structs
- `src/hyperdim.rs` - HVec10240 serialization
- `src/framework_ops.rs` - Import/export implementations
