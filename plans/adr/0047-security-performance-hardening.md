# ADR-0047: Security & Performance Hardening for v0.2.0

## Status

Accepted

## Context and Problem Statement

After comprehensive codebase analysis (Wave 13 analysis swarm, Feb 2026), 9 findings were identified across security, performance, and observability. The crate has been published as v0.1.0 and is now receiving external usage. Production hardening is needed before v0.2.0 to address critical security gaps and performance anti-patterns discovered during post-release review.

Phase 27 in GOAP_STATE.md tracks all 16 findings from the specialist analysis swarm. This ADR scopes the highest-ROI subset for immediate action.

## Decision Drivers

- Bincode deserialization has no size limit (CVE-class DoS vulnerability)
- Error variants lose source chain (thiserror 2.0 anti-pattern)
- Production code uses `expect()` which panics instead of returning `Result`
- `QueryCache` uses `Mutex` for read-heavy workload (perf anti-pattern)
- File operations accept user paths without validation (path traversal risk)

## Considered Options

### Option 1: Broad Phase 27 (all 16 items from GOAP_STATE)

Implement all 16 findings from the analysis swarm in a single sprint.

- Good, because comprehensive — eliminates all known issues
- Good, because single release addresses everything
- Bad, because too many changes in one release increases regression risk
- Bad, because mixes critical security fixes with nice-to-have optimizations
- Bad, because high cost (estimated 38 points) delays v0.2.0

### Option 2: Focused 5-item hardening sprint (Chosen)

Address the 5 highest-ROI items: S1 (bincode limits), S2 (path validation), O3 (error sources), P3 (cache RwLock), and the production `expect()` fix.

- Good, because eliminates the critical DoS vector (S1)
- Good, because preserves error chains for debugging (O3)
- Good, because small diff reduces regression risk
- Good, because unblocks v0.2.0 with confidence
- Bad, because defers performance optimizations (P1, P2)
- Bad, because observability coverage stays at ~30%

### Option 3: Defer all to post-v0.2.0

Ship v0.2.0 with no hardening changes.

- Good, because zero regression risk
- Bad, because S1 (bincode DoS) is a critical security vulnerability
- Bad, because external users are already consuming v0.1.0
- Bad, because error chain loss makes production debugging difficult

## Decision Outcome

Chosen option: "Focused 5-item hardening sprint", because it addresses the critical security vulnerability (bincode DoS) and the most impactful quality issues while keeping the change surface small enough for confident release.

### Positive Consequences

- Critical DoS vector (bincode deserialization) eliminated
- Error chains preserved for debugging via `#[source]` attributes
- No panics in production paths
- Concurrent cache reads enabled via `RwLock`
- File operation security improved with path validation

### Negative Consequences

- Performance optimizations (P1: `Arc<str>` concept IDs, P2: parallel `to_hypervector`) deferred to Phase 28
- Observability coverage stays at ~30% until next wave
- Remaining 11 findings from Phase 27 still open

## Implementation

### Action 1: `add_bincode_size_limits`

**Priority:** CRITICAL (security)
**File:** `src/framework_ops.rs`, `src/wasm.rs`
**Cost:** 3

Replace unbounded `bincode::deserialize()` with size-limited deserialization:

```rust
use bincode::Options;

const MAX_IMPORT_SIZE: u64 = 100 * 1024 * 1024; // 100 MB default

let options = bincode::DefaultOptions::new()
    .with_limit(MAX_IMPORT_SIZE);
let payload: ExportPayload = options.deserialize(&data)?;
```

- Default `MAX_IMPORT_SIZE`: 100 MB (configurable via `FrameworkBuilder`)
- Apply to `import_binary()` in both native and WASM paths
- Add test: oversized payload returns `MemoryError::ImportSizeExceeded`

### Action 2: `fix_error_source_attributes`

**Priority:** HIGH (correctness)
**File:** `src/error.rs`
**Cost:** 2

Change `Database(String)` and similar variants to wrap source errors where possible. Add `#[source]` attributes per thiserror 2.0 conventions:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("database error: {0}")]
    Database(#[source] libsql::Error),
    // ... other variants with #[source] where applicable
}
```

This preserves the full error chain for `anyhow`/`eyre` consumers and production log correlation.

### Action 3: `remove_production_expect`

**Priority:** HIGH (reliability)
**File:** `src/framework.rs:177`
**Cost:** 1

Replace `expect("reservoir initialized above")` with proper `Result` propagation:

```rust
// Before
let reservoir = self.reservoir.as_ref().expect("reservoir initialized above");

// After
let reservoir = self.reservoir.as_ref()
    .ok_or(MemoryError::ReservoirNotInitialized)?;
```

Add `ReservoirNotInitialized` variant to `MemoryError` if not already present.

### Action 4: `cache_mutex_to_rwlock`

**Priority:** MEDIUM (performance)
**File:** `src/singularity.rs`
**Cost:** 2

Replace `Mutex<QueryCache>` with `std::sync::RwLock<QueryCache>`:

```rust
// Before
query_cache: Mutex<QueryCache>,

// After
query_cache: std::sync::RwLock<QueryCache>,
```

- Cache lookups (`find_similar_cached`) use `read()` lock
- Cache invalidation and insertion use `write()` lock
- Read-heavy workloads benefit from concurrent reader access

### Action 5: `add_path_validation`

**Priority:** HIGH (security)
**File:** `src/framework_ops.rs`
**Cost:** 2

Add path canonicalization and validation before file operations:

```rust
fn validate_path(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize()
        .map_err(|e| MemoryError::InvalidPath(path.display().to_string(), e))?;

    // Reject path traversal attempts
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(MemoryError::PathTraversal(path.display().to_string()));
    }

    Ok(canonical)
}
```

- Apply to `export_json()`, `import_json()`, `export_binary()`, `import_binary()`, `backup()`, `restore()`
- Reject paths containing `..` components
- When a base directory is configured, ensure resolved path is within it

## GOAP Actions

| # | Action | Preconditions | Effects | Cost |
|---|--------|---------------|---------|------|
| 1 | `add_bincode_size_limits` | `export_import_functionality: true` | `bincode_size_limits_added: true` | 3 |
| 2 | `fix_error_source_attributes` | `error_context_improved: true` | `error_source_attributes_added: true` | 2 |
| 3 | `remove_production_expect` | `core_modules_created: true` | `production_expect_fixed: true` | 1 |
| 4 | `cache_mutex_to_rwlock` | `concept_cache_implemented: true` | `cache_rwlock_fixed: true` | 2 |
| 5 | `add_path_validation` | `export_import_functionality: true` | `path_traversal_protection_added: true` | 2 |

**Total cost:** 10

## Follow-up Actions (Deferred to Phase 28)

| ID | Item | Reason for Deferral |
|----|------|---------------------|
| P1 | `Arc<str>` concept IDs | Performance optimization, not blocking |
| P2 | Parallel `to_hypervector` | Performance optimization, not blocking |
| O1 | Tracing coverage expansion (30% → 70%) | Incremental improvement, not blocking |

## References

- GOAP_STATE.md Phase 27 tracking fields
- ADR-0045: Security Input Validation (related)
- ADR-0035: Cache Memory Guardrails (related)
- thiserror 2.0 migration guide

---

**Created:** 2026-02-26
**Author:** Security & Performance Analysis Swarm
**Related:** ADR-0042 (Release Automation), ADR-0045 (Input Validation)
