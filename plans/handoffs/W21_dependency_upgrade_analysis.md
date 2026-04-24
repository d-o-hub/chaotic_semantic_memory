# Wave 21: Dependency Upgrade Analysis

**Date:** 2026-04-24
**Scope:** IQ-01 to IQ-04 dependency upgrade evaluation

## Summary

Most dependency upgrades have already been completed in prior waves:

| Task | Target | Current | Status |
|------|--------|---------|--------|
| IQ-01: libsql upgrade | 0.9.x | 0.9.30 | ✅ Complete |
| IQ-02: rand upgrade | 0.9.x | 0.10.1 | ✅ Complete (ahead of target) |
| IQ-03: getrandom wasm | wasm_js feature | 0.4.2 with wasm_js | ✅ Complete |
| IQ-04: bincode→postcard | postcard | 1.3.3 bincode | ⏳ Pending |

## IQ-04: bincode→postcard Migration

**Current:** bincode 1.3.3
**Advisory:** RUSTSEC-2025-0141 (development ceased)

### Analysis

1. **bincode 1.3.3 is stable and widely used** - the advisory notes development has ceased but no active vulnerabilities
2. **postcard is smaller and WASM-friendly** - better for embedded/WASM targets
3. **Breaking changes:**
   - Different encoding format (not interchangeable)
   - Requires schema changes for persisted data
   - Migration would break backward compatibility with existing databases

### Recommendation

**Deferred** until v0.4.0 release:
- Requires export/import migration tool for users with existing databases
- Current bincode 1.3.3 is stable and functional
- Schedule for major version bump when breaking changes are acceptable

### Migration Complexity

| Aspect | Complexity |
|--------|------------|
| Code changes | Medium (API differs) |
| Data migration | High (requires tooling) |
| WASM impact | Positive (smaller) |
| Testing | High (serialization tests) |

**Estimated cost:** 16 developer hours

## Conclusion

IQ-01 to IQ-03 are already complete. IQ-04 (bincode→postcard) is deferred to v0.4.0 due to backward compatibility concerns.