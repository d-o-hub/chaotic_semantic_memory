---
name: swarm-testing-quality
description: "Property-based testing, fuzzing, and edge case coverage. Use when adding comprehensive test coverage with proptest or cargo-fuzz."
---

# Swarm: Testing & Quality

## Workflow
1. Check current test coverage in `src/*/tests` modules
2. Identify properties to test (invariants, roundtrips, bounds)
3. Add `proptest` dependency to `Cargo.toml`
4. Create `tests/property_based.rs` with property tests
5. Set up `fuzz/` directory with cargo-fuzz targets
6. Add edge case tests to module test blocks
7. Run validation gates

## Key Properties to Test

### HVec10240
- `from_bytes(to_bytes(v)) == v` (roundtrip)
- `cosine_similarity(v, v) == 1.0` (self-similarity)
- `cosine_similarity(a, b) == cosine_similarity(b, a)` (symmetry)
- `cosine_similarity(a, b)` in `[-1.0, 1.0]` (bounds)

### Reservoir
- `reset()` clears state to zeros
- `step()` with same input produces same output after `reset()`
- `to_hypervector()` fails if `size < 10240`

### Persistence
- `save_concept(c); load_concept(c.id) == Some(c)` (roundtrip)
- `delete_concept(id); load_concept(id) == None` (deletion)
- FK constraints reject invalid associations

## Commands

```bash
# Run property tests
cargo test --test property_based

# Run fuzzer (requires cargo-fuzz)
cargo fuzz run fuzz_hvec_from_bytes
```

## LOC Constraint
All files must remain ≤ 500 lines. Create new test files rather than inflating existing ones.
