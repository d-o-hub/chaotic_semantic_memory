# [ADR-0053] Wave 15: API Hardening, Missing Features & New Capabilities

## Status
Proposed

## Context and Problem Statement

Comprehensive analysis of the codebase (Wave 14 complete, v0.1.3) revealed 15 actionable findings across 6 categories:
1. **Production safety issues**: `unwrap()` in reservoir hot path, semaphore deadlock risk in persistence, silent error swallowing
2. **Missing API surface**: no `update_concept` at framework level, no `disassociate`, no `bundle_concepts_strict`, no `clear_cache`
3. **Error handling gaps**: `row.get().unwrap_or(0)` corrupts version history, `current_dir().unwrap_or_default()` weakens path validation
4. **Performance**: concept ID cloning in similarity queries, redundant dimension checks
5. **Documentation**: reservoir invariants, persistence schema semantics, load/merge behavior undocumented
6. **WASM parity**: concept_history, update_concept, disassociate not exposed

## Decision Drivers
- Production reliability: eliminate remaining panic paths in library code
- API completeness: users need update, disassociate, cache control
- Error fidelity: silent swallowing of DB errors corrupts state
- AGENTS.md: 500 LOC limit, all public APIs return Result<T, Error>

## Considered Options
- **Option A**: Fix only critical safety issues (scope: small)
- **Option B**: Full API hardening + new features + doc pass (scope: medium)
- **Option C**: Option B + WASM IndexedDB persistence (scope: large)

## Decision Outcome
Chosen option: **Option B** — full API hardening and new features, deferring WASM IndexedDB to post-1.0.

### Phase 32: Production Safety (cost: 6)
1. **Replace `try_into().unwrap()` in reservoir** (reservoir.rs:323) — build `[u128; 80]` directly
2. **Fix semaphore deadlock in persistence** — `init_schema` → `apply_migrations` → `schema_version` nests 3 `acquire_remote_slot` calls; refactor to pass connection through internal methods
3. **Fix `row.get().unwrap_or(0)`** in `record_concept_version` (persistence.rs:464) — map to `MemoryError::Database`
4. **Fix `current_dir().unwrap_or_default()`** in `validate_path` (framework_ops.rs:50) — return error on failure

### Phase 33: API Completeness (cost: 10)
1. **Add `update_concept_vector(id, vector)`** to framework — persists + records version
2. **Add `update_concept_metadata(id, metadata)`** to framework
3. **Add `disassociate(from, to)`** to singularity + framework + persistence
4. **Add `clear_associations(from)`** to singularity + framework + persistence
5. **Add `bundle_concepts_strict(ids)`** to singularity — returns `NotFound` for missing IDs
6. **Add `clear_similarity_cache()`** to singularity — public cache control
7. **Add `with_version_retention(n)`** to `FrameworkBuilder`

### Phase 34: Error Handling Hardening (cost: 4)
1. **Add `#[source]` attributes** to `MemoryError::Database` and `MemoryError::Reservoir` for error chain support
2. **Replace `persistence.size().await.unwrap_or(0)`** in `stats()` with `Option<u64>`
3. **Remove dead dimension check** in `Singularity::inject` (data.len() == 80 is compile-time true)

### Phase 35: Documentation Pass (cost: 4)
1. **Document reservoir invariants** — input_size, partial update stride, spectral radius
2. **Document persistence schema** — version retention, migration semantics
3. **Document load_replace vs load_merge** behavior in framework
4. **Add WASM parity notes** to lib.rs module docs

### Phase 36: WASM API Parity (cost: 4)
1. **Expose `update_concept`** to WASM
2. **Expose `disassociate`** to WASM
3. **Expose `concept_count` / `stats`** to WASM
4. **Document WASM persistence story** (bytes export → IndexedDB)

### Positive Consequences
- Zero `unwrap()` in non-test library code
- Complete CRUD API surface (create, read, update, delete + associations)
- Error chains preserved for debugging via `#[source]`
- Semaphore deadlock eliminated for remote Turso deployments

### Negative Consequences
- ~5 new public methods increase API surface
- FrameworkStats breaking change: `db_size_bytes: u64` → `db_size_bytes: Option<u64>`
- WASM binary size may increase ~5KB with new bindings

## Implementation Priority
P1 (Critical): Phase 32 items 1-4 (safety)
P2 (High): Phase 33 items 1-4 (update + disassociate)
P3 (Medium): Phase 33 items 5-7, Phase 34, Phase 36
P4 (Low): Phase 35 (docs)

## Validation Criteria
- `cargo test --all-features` passes
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- No `unwrap()` in `src/*.rs` outside `#[cfg(test)]` blocks
- All new APIs have at least one integration test
- All files under 500 LOC
