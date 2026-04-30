# GOAP Plan: Code Coverage Improvement

**Date**: 2026-04-29
**Orchestrator**: goap_coverage_analysis_2026_04_29
**Baseline**: 534 tests, 7,838 LOC tests, 9,467 LOC source

---

## Current World State Summary

| Metric | Value |
|--------|-------|
| Total tests | 534 |
| Test files | 46 |
| Source files | 55 (src/*.rs) |
| Test LOC | 7,838 |
| Source LOC | 9,467 |
| Test:Source ratio | 0.83:1 |
| Inline test blocks | 16 modules |
| Dedicated test files | 45 files |
| Dead code markers | 4 (`#[allow(dead_code)]`) |
| `todo!()` markers | 0 |
| `unwrap()` outside tests | 0 (all in #[cfg(test)] or doc examples) |

---

## Coverage Gap Analysis

### A. Modules Without Inline Tests (21)

| Module | Has Dedicated Test | Status |
|--------|--------------------|--------|
| encoder | ✅ encoder_tests.rs | Covered |
| framework | ✅ framework_lifecycle.rs, framework_core.rs | Covered |
| framework_builder | ✅ builder_config*.rs | Covered |
| framework_events | ✅ (via framework tests) | Covered |
| framework_ops | ✅ framework_ops_coverage.rs | Covered |
| framework_ttl | ✅ ttl_lifecycle.rs | Covered |
| hyperdim_simd | ✅ hyperdim_coverage.rs | Covered |
| lib | ✅ (prelude tests) | Covered |
| persistence | ✅ persistence_crud.rs | Covered |
| persistence_migrations | ✅ migration_errors.rs | Covered |
| persistence_ops | ✅ persistence_ops_coverage.rs | Covered |
| persistence_versions | ✅ persistence_versions_coverage.rs | Covered |
| persistence_wasm | ✅ (via persistence tests) | Covered |
| reservoir | ✅ reservoir_determinism.rs | Covered |
| reservoir_inertial | ✅ reservoir_inertial_tests.rs | Covered |
| reservoir_sparse | ✅ (via reservoir tests) | Covered |
| singularity | ✅ singularity tests in various | Covered |
| singularity_cache | ✅ cache*.rs | Covered |
| singularity_retrieval | ✅ retrieval*.rs | Covered |
| wasm | ✅ (JS smoke test in CI) | Covered |
| wasm_ext | ✅ (via wasm tests) | Covered |

**All modules have test coverage.**

### B. Test-to-Source LOC Ratios (lowest 5)

| Module | Source LOC | Test LOC | Ratio | Priority |
|--------|------------|----------|-------|----------|
| graph_traversal | 463 | 2,423* | 5.23 | LOW |
| hyperdim | 486 | 2,558* | 5.26 | LOW |
| framework_ops | 480 | 2,662* | 5.54 | LOW |
| bridge_retrieval | 386 | 2,423* | 6.27 | LOW |
| framework | 492 | 3,170* | 6.44 | LOW |

*Note: Ratios appear high due to counting all tests/*coverage*.rs files for each module.

### C. Actual Coverage Gaps

| Gap | Module | Issue | Priority | Cost |
|-----|--------|-------|----------|------|
| C1 | `hyperdim_simd.rs` | AVX2/NEON paths not tested inline | HIGH | 3 |
| C2 | `reservoir_sparse.rs` | Sparse matrix edge cases (empty rows) | MED | 2 |
| C3 | `persistence_wasm.rs` | WASM persistence stubs have no unit tests | MED | 2 |
| C4 | `framework_events.rs` | MemoryEvent emission not verified | MED | 2 |
| C5 | `singularity_cache.rs` | Cache eviction edge cases (LRU pressure) | LOW | 1 |

---

## GOAP Actions

### Target State
```yaml
coverage_improved: true
hyperdim_simd_avx2_tests: true
reservoir_sparse_edge_tests: true
persistence_wasm_stub_tests: true
framework_events_emission_tests: true
singularity_cache_lru_pressure_tests: true
coverage_tests_added: 15
```

### Ordered Actions (by priority)

```yaml
actions:
  # ─── PHASE 1: SIMD Path Testing (cost: 3) ───
  - name: add_hyperdim_simd_inline_tests
    preconditions:
      core_modules_created: true
    effects:
      hyperdim_simd_avx2_tests: true
    cost: 3
    status: complete
    completed_at: "2026-04-30"
    file: src/hyperdim_simd.rs
    description: |
      Add #[cfg(test)] block to hyperdim_simd.rs with:
      1. AVX2 path validation (x86_64 target)
      2. NEON path validation (aarch64 target)
      3. Scalar fallback correctness
      Use #[cfg(target_arch = "x86_64")] guards for arch-specific tests.

  # ─── PHASE 2: Sparse Matrix Edge Cases (cost: 2) ───
  - name: add_reservoir_sparse_edge_tests
    preconditions:
      reservoir_dense_matrix_infeasible: false
    effects:
      reservoir_sparse_edge_tests: true
    cost: 2
    status: complete
    completed_at: "2026-04-30"
    file: src/reservoir_sparse.rs, tests/reservoir_determinism.rs
    description: |
      Add edge case tests for sparse reservoir matrix:
      1. Empty row handling (zero connections)
      2. Single connection row
      3. Max degree row (k=64)
      4. Zero-weight edge filtering

  # ─── PHASE 3: WASM Stub Testing (cost: 2) ───
  - name: add_persistence_wasm_stub_tests
    preconditions:
      wasm_compiles: true
    effects:
      persistence_wasm_stub_tests: true
    cost: 2
    status: complete
    completed_at: "2026-04-30"
    file: src/persistence_wasm.rs
    description: |
      Add #[cfg(test)] block testing WASM persistence stub behavior:
      1. Stub returns appropriate error for unsupported operations
      2. Serialization compatibility with non-WASM format

  # ─── PHASE 4: Memory Event Testing (cost: 2) ───
  - name: add_framework_events_emission_tests
    preconditions:
      memory_event_enum_created: true
    effects:
      framework_events_emission_tests: true
    cost: 2
    status: complete
    completed_at: "2026-04-30"
    file: tests/framework_lifecycle.rs
    description: |
      Add tests verifying MemoryEvent emission:
      1. Inject emits Created event
      2. Update emits Updated event
      3. Delete emits Deleted event
      4. Associate emits Associated event

  # ─── PHASE 5: Cache LRU Pressure Testing (cost: 1) ───
  - name: add_singularity_cache_lru_tests
    preconditions:
      concept_cache_implemented: true
    effects:
      singularity_cache_lru_pressure_tests: true
    cost: 1
    status: complete
    completed_at: "2026-04-30"
    file: tests/cache_lru.rs
    description: |
      Add edge case tests for LRU cache:
      1. Cache pressure eviction (fill then access old)
      2. Cache invalidation on concept delete
      3. Cache coherency with persistence reload
```

### Dependency Graph

```
add_hyperdim_simd_inline_tests ────┐
add_reservoir_sparse_edge_tests ───┤
add_persistence_wasm_stub_tests ───┤──→ (no deps, parallel)
add_framework_events_emission_tests ┤
add_singularity_cache_lru_tests ───┘
```

---

## Success Criteria (All Complete ✓)

- [x] AVX2 SIMD path tested inline
- [x] NEON SIMD path tested inline (aarch64)
- [x] Sparse reservoir edge cases covered
- [x] WASM persistence stub behavior verified
- [x] MemoryEvent emission verified
- [x] LRU cache pressure tested
- [x] All tests pass (cargo test --all-features)
- [x] No new dead_code markers
- [x] Coverage ratio maintains >0.80

---

## Next Steps

1. **Start**: `add_hyperdim_simd_inline_tests` (highest priority)
2. **Parallel**: All 5 actions can run concurrently
3. **Validate**: Run full test suite after each action
4. **Update**: Update GOAP_STATE.md with completed actions