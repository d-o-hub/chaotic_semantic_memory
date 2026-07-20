# Swarm Group D Analysis: Advanced Features Assessment

**Date:** 2026-02-17  
**Group:** D (Advanced Features)  
**Scope:** Missing capabilities, WASM completeness, production readiness

---

## 1. Missing Capabilities Analysis

### 1.1 GOALS.md Status Audit

All Phase 8 Advanced Features goals are **COMPLETED** (marked true in both GOALS.md and GOAP_STATE.md):

| Goal | Status | Implementation |
|------|--------|----------------|
| export_import_functionality | true | `framework_ops.rs`: export_json, import_json, export_binary |
| concept_versioning_enabled | true | `persistence.rs`: concept_versions table, get_concept_history |
| schema_migration_support | true | `persistence.rs`: __schema_version, apply_migrations |
| backup_restore_operations | true | `framework_ops.rs`: backup, restore using VACUUM INTO |

### 1.2 False/Missing Goals from Optimization Section

The following optimization goals remain **false** and represent technical debt:

**Phase 1: Correctness (Critical)**
- `permute_shift_zero_bug` - Hyperdimension permutation edge case
- `reservoir_to_hvec_div_zero` - Division by zero in reservoir projection
- `associations_allow_duplicates` - Duplicate association handling
- `load_silently_overwrites` - Data loss on load semantics
- `reservoir_not_reset_between_sequences` - State leakage between sequences
- `sqlite_foreign_keys_not_enforced` - Referential integrity gaps
- `conceptbuilder_swallows_metadata_errors` - Error propagation issues
- `libsql_deprecated_apis_used` - API modernization needed

**Phase 2: Performance (High Impact)**
- `reservoir_dense_matrix_infeasible` - See ADR-0004 (sparse matrix implemented)
- `singularity_search_sequential` - See ADR-0007 (parallel search implemented)
- `reservoir_step_per_alloc` - Allocation optimization opportunity
- `bundle_per_chunk_alloc` - Batch allocation optimization
- `persistence_no_batching` - See ADR-0006 (batching implemented)
- `persistence_connection_unsafe` - See ADR-0005 (connection pooling implemented)

**Phase 3: Capabilities (Medium Impact)**
- `wasm_rayon_not_gated` - See ADR-0008 (proper gating implemented)
- `no_concept_deletion_in_framework` - **COMPLETED**: delete_concept exists
- `no_memory_limits` - **COMPLETED**: max_concepts, max_associations_per_concept
- `prelude_module_missing` - **COMPLETED**: prelude module exists
- `no_integration_tests` - **COMPLETED**: 7 integration tests passing

### 1.3 Remediation Plans for False Goals

**Critical Priority (Phase 1)**:
1. Fix reservoir state reset in `process_sequence()` - call reset() before loop
2. Add division-by-zero guards in `to_hypervector()` chunk processing
3. Validate foreign key enforcement in `init_schema()` - PRAGMA already set, verify behavior
4. Review load semantics to ensure merge vs replace is explicit

**Performance Priority (Phase 2)**:
These are mostly **COMPLETED** per GOAP_STATE.md - the goals file may be stale:
- Sparse reservoir matrix: Implemented (ADR-0004)
- Parallel search: Implemented with rayon (ADR-0007)
- Connection pooling: Implemented (ADR-0005)
- Batch operations: Implemented (ADR-0006)

---

## 2. WASM API Completeness Audit

### 2.1 Current WASM API Surface (`src/wasm.rs`)

**Implemented (166 LOC)**:
- `WasmFramework::new()` - Constructor without persistence
- `inject_concept(id, vector)` - Single concept injection
- `probe(vector, top_k)` -> Array of {id, score} - Similarity search
- `associate(from, to, strength)` - Create association
- `delete_concept(id)` - Remove concept
- `get_associations(id)` -> Array of {to, strength} - Retrieve associations
- `metrics_snapshot()` -> Object with counters - Runtime metrics
- `stats()` -> {concept_count, db_size_bytes} - Storage stats
- `random_hypervector()` -> bytes[1280] - Utility
- `cosine_similarity(a, b)` -> f32 - Utility

### 2.2 Missing WASM APIs (Native-Only)

| Native API | WASM Status | Gap Severity | Notes |
|------------|-------------|--------------|-------|
| `process_sequence()` | NOT EXPOSED | **HIGH** | Reservoir temporal processing unavailable in WASM |
| `inject_concepts()` (batch) | NOT EXPOSED | MEDIUM | Batch injection unavailable |
| `associate_many()` (batch) | NOT EXPOSED | MEDIUM | Batch associations unavailable |
| `probe_batch()` | NOT EXPOSED | MEDIUM | Batch queries unavailable |
| `export_json()` | STUB ONLY | **HIGH** | Returns error - persistence unavailable |
| `import_json()` | STUB ONLY | **HIGH** | Returns error - persistence unavailable |
| `backup()` | STUB ONLY | LOW | Expected - WASM has no filesystem |
| `restore()` | STUB ONLY | LOW | Expected - WASM has no filesystem |
| `concept_history()` | STUB ONLY | MEDIUM | Versioning API unavailable |
| `load_replace()` / `load_merge()` | NOT EXPOSED | LOW | In-memory only in WASM |
| `persist()` | NOT EXPOSED | LOW | Checkpoint not applicable |
| `persistence_health_check()` | NOT EXPOSED | LOW | Health check not applicable |

### 2.3 WASM Binary Size Analysis

Per W5_C_to_D_wasm_size_report.md:
- **Current size:** Under 500KB target
- **Features included:** Core framework, reservoir, singularity
- **Excluded:** libsql/tokio I/O (replaced with stubs)

### 2.4 WASM API Recommendations

**High Priority:**
1. **Expose `process_sequence()`** - Temporal processing is core to chaotic semantics
   - Add to `WasmFramework` with proper JS Array input handling
   - Estimated +30 LOC, minimal size impact

2. **Memory-based Export/Import** - Instead of file paths, use JS Uint8Array
   - `export_to_bytes()` -> Uint8Array
   - `import_from_bytes(data, merge)` 
   - Enables data portability in WASM without filesystem

**Medium Priority:**
3. **Batch operations** - `inject_concepts_batch()`, `associate_many()`
   - Improves throughput for bulk operations
   - Consistent with native API parity goals (ADR-0022)

---

## 3. Advanced Feature Proposals

### 3.1 Proposal A: Concept Expiration (TTL)

**Motivation:** Production systems need automatic cleanup of stale data

**Design:**
```rust
// In FrameworkConfig
pub concept_ttl_seconds: Option<u64>,  // None = no expiration

// In Concept
pub expires_at: Option<u64>,

// New APIs
pub async fn inject_concept_with_ttl(
    &self, 
    id: impl Into<String>, 
    vector: HVec10240,
    ttl_seconds: u64
) -> Result<()>

pub async fn cleanup_expired(&self) -> Result<usize>  // Returns count removed
```

**Storage:**
- Add `expires_at INTEGER` column to concepts table
- Index on expires_at for efficient cleanup queries

**WASM Impact:** 
- Fully compatible - no persistence required
- Background cleanup can be triggered on operation

**Size Impact:** ~50 bytes binary increase

**ADR Required:** Yes - ADR-0024

---

### 3.2 Proposal B: Weighted Forgetting (Gradual Decay)

**Motivation:** Biological memory fades gradually; associations should weaken over time

**Design:**
```rust
// In FrameworkConfig
pub association_decay_halflife_seconds: Option<u64>,  // None = no decay

// In Singularity
pub fn apply_decay(&mut self, elapsed_seconds: u64) {
    // strength *= 0.5^(elapsed / halflife)
}

// Automatic decay on access
pub fn get_associations(&self, id: &str) -> Vec<(String, f32)> {
    // Apply decay calculation to returned strengths
}
```

**Decay Formula:**
```
strength_t = strength_0 * exp(-lambda * t)
where lambda = ln(2) / halflife
```

**Storage:**
- Store last_accessed timestamp for each association
- Calculate decay on read (lazy evaluation)

**WASM Impact:** Fully compatible

**ADR Required:** Yes - ADR-0025

---

### 3.3 Proposal C: Namespace Isolation (Soft Multi-Tenancy)

**Motivation:** Single deployment serving multiple users/agents without data leakage

**Design:**
```rust
pub struct NamespacedFramework {
    namespace: String,
    inner: ChaoticSemanticFramework,
}

// Concept IDs are prefixed internally: "{namespace}::{user_id}"
// Queries only return concepts from same namespace
```

**Alternative - Built-in to Framework:**
```rust
// In FrameworkConfig
pub namespace: Option<String>,

// All operations automatically prefix/sandbox
pub async fn inject_concept(&self, id: impl Into<String>, ...) {
    let namespaced_id = format!("{}::{}", self.namespace, id.into());
    // ...
}
```

**Storage:**
- Add `namespace TEXT` column to concepts table
- Composite index on (namespace, id)
- Query filter: `WHERE namespace = ?1`

**WASM Impact:** Fully compatible

**ADR Required:** Yes - ADR-0026

---

## 4. Production Operational Features

### 4.1 Missing Operational Capabilities

| Feature | Status | Priority | Implementation Notes |
|---------|--------|----------|---------------------|
| Health check endpoint | Partial | HIGH | `persistence_health_check()` exists but no composite health |
| Graceful degradation | Missing | HIGH | No circuit breaker for Turso failures |
| Rate limiting | Missing | MEDIUM | No query throttling |
| Operational metrics | Partial | MEDIUM | Basic counters exist, no histograms |
| Alert thresholds | Missing | MEDIUM | No configurable limits |
| Hot reload config | Missing | LOW | Config is build-time only |
| Distributed tracing | Missing | LOW | No span context propagation |

### 4.2 Recommended Production Additions

**4.2.1 Composite Health Check**
```rust
pub struct HealthStatus {
    pub overall: Health,
    pub persistence: Option<Health>,  // None if no persistence
    pub memory_usage: Health,
    pub reservoir: Health,
}

pub async fn health_check(&self) -> HealthStatus {
    // Check all subsystems, return degraded if any component failing
}
```

**4.2.2 Circuit Breaker for Persistence**
```rust
pub struct PersistenceCircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicU64,  // timestamp
    state: AtomicU8,  // Closed, Open, HalfOpen
}

// On persistence failure, increment counter
// After threshold, enter Open state - skip persistence attempts
// After timeout, enter HalfOpen - try one request
```

**4.2.3 Metrics Histograms**
```rust
pub struct FrameworkMetrics {
    // Existing counters...
    
    // New histograms (using quantile sketches)
    probe_latency_histogram: RwLock<HdrHistogram>,
    concept_size_histogram: RwLock<HdrHistogram>,
}
```

### 4.3 Configuration Management

Current: Build-time constants in code  
Needed: Runtime configuration with defaults

```rust
pub struct OperationalConfig {
    // Health checks
    pub health_check_interval_secs: u64,
    pub health_timeout_ms: u64,
    
    // Circuit breaker
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout_secs: u64,
    
    // Resource limits
    pub max_query_time_ms: u64,
    pub max_concurrent_queries: usize,
    
    // Observability
    pub metrics_buffer_size: usize,
    pub slow_query_threshold_ms: u64,
}
```

---

## 5. Summary and Recommendations

### 5.1 Immediate Actions

1. **Fix Critical Correctness Issues** (Phase 1 false goals)
   - Reservoir reset between sequences
   - Division by zero guards
   - Load semantics clarity

2. **Complete WASM Parity**
   - Expose `process_sequence()` to WASM
   - Add memory-based export/import (Uint8Array)
   - Add batch operations

3. **Add Production Health Checks**
   - Composite health status
   - Circuit breaker for persistence

### 5.2 Short-Term (Next Sprint)

1. **Draft ADR-0024: Concept Expiration (TTL)**
   - High value for production deployments
   - Minimal implementation complexity
   - Full WASM compatibility

2. **Implement Configuration Management**
   - Move from constants to runtime config
   - Environment variable support
   - Sensible defaults

### 5.3 Medium-Term (Next Quarter)

1. **Draft ADR-0025: Weighted Forgetting**
2. **Draft ADR-0026: Namespace Isolation**
3. **Add Operational Metrics Dashboard**

### 5.4 Alignment Check

All proposals respect constraints:
- Source files <= 500 LOC: Yes, additions are small and focused
- WASM binary < 500KB: Yes, TTL and decay are lightweight
- Backward compatibility: Yes, all features are opt-in via config
- libsql (not turso-client): Yes, no new database dependencies

---

## Appendix: Module LOC Audit

From GOAP_STATE.md:
- lib.rs: 35 (target: <500) - OK
- error.rs: 32 - OK
- hyperdim.rs: 410 - OK
- reservoir.rs: 428 - OK
- singularity.rs: 440 - OK
- persistence.rs: 499 - OK (at limit)
- persistence_wasm.rs: 110 - OK
- framework.rs: 495 - OK
- framework_ops.rs: 212 - OK
- framework_validation.rs: 81 - OK
- wasm.rs: 166 - OK

**Headroom:** persistence.rs is at 499 LOC. New persistence features should go in persistence_ops.rs (262 LOC, has room).

---

**Analysis Complete**  
**Next Step:** Create ADR-0024 for Concept Expiration if approved
