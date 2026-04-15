# Memory Leak Prevention Analysis Report

**Swarm Groups**: B (Performance) & D (Advanced Features)  
**Date**: 2026-02-20  
**Scope**: Full codebase analysis for memory safety and leak risks  
**Severity Levels**: Critical | High | Medium | Low

---

## Executive Summary

The chaotic_semantic_memory codebase shows generally good memory hygiene with Rust's ownership system providing baseline safety. However, several areas require attention to prevent unbounded memory growth, resource exhaustion, and potential leaks in long-running scenarios.

**Overall Risk Assessment**: MEDIUM  
**Critical Issues**: 1  
**High Issues**: 4  
**Medium Issues**: 6  
**Low Issues**: 3

---

## 1. Resource Leaks

### 1.1 Database Connection Management - MEDIUM

**Location**: `src/persistence.rs:72-94`

```rust
pub(crate) async fn connect(&self) -> Result<Connection> {
    let conn = self
        .db
        .connect()
        .map_err(|e| MemoryError::Database(format!("Failed to connect: {}", e)))?;

    conn.execute("PRAGMA foreign_keys = ON;", ())
        .await
        .map_err(|e| MemoryError::Database(format!("Failed to enable foreign keys: {}", e)))?;

    Ok(conn)
}
```

**Issue**: The `Connection` object returned from `connect()` implements `Drop` for cleanup, but in async contexts with high concurrency, connections may not be dropped immediately when the future is cancelled. The libsql crate manages connection pooling internally, but explicit connection closure is not enforced.

**Remediation**:
```rust
pub(crate) async fn connect(&self) -> Result<Connection> {
    let conn = self
        .db
        .connect()
        .map_err(|e| MemoryError::Database(format!("Failed to connect: {}", e)))?;

    // Use scopeguard pattern for guaranteed cleanup
    let result = conn.execute("PRAGMA foreign_keys = ON;", ()).await;
    if let Err(e) = result {
        // Force close on setup failure
        drop(conn);
        return Err(MemoryError::Database(format!("Failed to enable foreign keys: {}", e)));
    }

    Ok(conn)
}
```

---

### 1.2 Semaphore Permit Release - LOW

**Location**: `src/persistence.rs:85-94`

```rust
pub(crate) async fn acquire_remote_slot(&self) -> Result<Option<OwnedSemaphorePermit>> {
    match &self.remote_limit {
        Some(limit) => {
            limit.clone().acquire_owned().await.map(Some).map_err(|e| {
                MemoryError::Database(format!("Failed to acquire pool slot: {}", e))
            })
        }
        None => Ok(None),
    }
}
```

**Issue**: `OwnedSemaphorePermit` correctly implements `Drop` to release the permit. However, if a task holding a permit is aborted or panics, the permit may not be released until the `OwnedSemaphorePermit` is dropped. This is generally safe due to RAII, but in extreme cases of task abortion, permits could be temporarily unavailable.

**Status**: ACCEPTABLE - RAII handles this correctly, but document the behavior.

---

### 1.3 Arc Cycles - LOW

**Location**: `src/framework.rs:16-22`

```rust
pub struct ChaoticSemanticFramework {
    pub(crate) singularity: Arc<RwLock<Singularity>>,
    pub(crate) persistence: Option<Arc<Persistence>>,
    pub(crate) reservoir: Arc<RwLock<Option<ChaoticReservoir>>>,
    pub(crate) config: FrameworkConfig,
    pub(crate) metrics: Arc<FrameworkMetrics>,
}
```

**Analysis**: No reference cycles detected. The architecture uses Arc for shared ownership but no circular references exist between Singularity, Persistence, or Framework. All Arc references are tree-structured.

**Status**: CLEAN - No action required.

---

## 2. Memory Growth

### 2.1 Unbounded Concept Storage - HIGH

**Location**: `src/singularity.rs:386-409`

```rust
fn evict_oldest_if_needed(&mut self) {
    let Some(limit) = self.config.max_concepts else {
        return;  // NO LIMIT - unbounded growth!
    };
    // ... eviction logic
}
```

**Issue**: By default, `max_concepts` is `None`, allowing unbounded concept storage growth. In long-running applications, this will eventually exhaust system memory.

**Remediation**:
```rust
// In src/framework_builder.rs
const DEFAULT_MAX_CONCEPTS: usize = 100_000;

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            // ...
            max_concepts: Some(DEFAULT_MAX_CONCEPTS), // Set safe default
            // ...
        }
    }
}

// In src/singularity.rs
const HARD_MAX_CONCEPTS_LIMIT: usize = 10_000_000; // Absolute ceiling

fn evict_oldest_if_needed(&mut self) {
    let limit = self.config.max_concepts
        .unwrap_or(DEFAULT_MAX_CONCEPTS)
        .min(HARD_MAX_CONCEPTS_LIMIT);
    // ... rest of eviction logic
}
```

---

### 2.2 Association Storage Growth - HIGH

**Location**: `src/singularity.rs:304-331`

```rust
pub fn associate(&mut self, from: &str, to: &str, strength: f32) -> Result<()> {
    // ...
    let links = self.associations.entry(from.to_string()).or_default();
    links.insert(to.to_string(), strength);

    if let Some(limit) = self.config.max_associations_per_concept {
        while links.len() > limit {
            // eviction logic
        }
    }
    // If limit is None, associations grow unbounded per concept!
}
```

**Issue**: Similar to concept storage, associations can grow unbounded per concept when `max_associations_per_concept` is `None`.

**Remediation**: Apply same default limit pattern as concepts.

---

### 2.3 Version History Without Retention Limits - CRITICAL

**Location**: `src/persistence.rs:450-498`

```rust
async fn record_concept_version(&self, conn: &Connection, concept: &Concept) -> Result<()> {
    // ... creates new version record
    
    conn.execute(
        "DELETE FROM concept_versions
         WHERE concept_id = ?1
         AND version <= (
            SELECT MAX(version) - ?2 FROM concept_versions WHERE concept_id = ?1
         )",
        params![concept.id.clone(), self.version_retention as i64],
    )
    // ...
}
```

**Issue**: The `version_retention` field defaults to 10, which is reasonable. However, there's NO hard limit enforcement at the database level. If the DELETE fails or is bypassed, versions accumulate indefinitely. Additionally, frequent updates to the same concept can create database bloat.

**Remediation**:
```rust
// Add hard limit check in record_concept_version
const MAX_VERSION_RETENTION: usize = 1000;

async fn record_concept_version(&self, conn: &Connection, concept: &Concept) -> Result<()> {
    // Enforce hard ceiling
    let retention = self.version_retention.min(MAX_VERSION_RETENTION);
    
    // First check total version count
    let count: i64 = conn.query(
        "SELECT COUNT(*) FROM concept_versions WHERE concept_id = ?1",
        params![concept.id.clone()]
    ).await?.next().await?.get(0)?;
    
    if count >= retention as i64 * 2 {
        // Emergency cleanup if somehow exceeded
        conn.execute(
            "DELETE FROM concept_versions WHERE concept_id = ?1
             ORDER BY version ASC LIMIT ?2",
            params![concept.id.clone(), count - retention as i64]
        ).await?;
    }
    // ... rest of logic
}
```

---

### 2.4 Metadata Size Without Limits - HIGH

**Location**: `src/concept_builder.rs:55-68`

```rust
pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
    if self.metadata_error.is_none() {
        match serde_json::to_value(value) {
            Ok(value) => {
                self.metadata.insert(key.into(), value);  // No size limit here!
            }
            // ...
        }
    }
    self
}
```

**Issue**: During concept building, there's no metadata size validation. While `validate_metadata_bytes` exists in `framework_validation.rs`, it's only called during framework injection, not during direct Singularity usage.

**Remediation**: Add builder-level metadata size tracking:
```rust
pub struct ConceptBuilder {
    // ...
    metadata_size_bytes: usize,
    const MAX_METADATA_BUILD_SIZE: usize = 10_000_000; // 10MB hard limit
}

pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
    // ... existing validation ...
    if let Ok(value) = serde_json::to_value(value) {
        let key = key.into();
        let value_size = key.len() + serde_json::to_string(&value).unwrap_or_default().len();
        
        if self.metadata_size_bytes + value_size > Self::MAX_METADATA_BUILD_SIZE {
            self.metadata_error = Some(MemoryError::InvalidInput {
                field: "metadata".to_string(),
                reason: format!("metadata exceeds {} bytes", Self::MAX_METADATA_BUILD_SIZE),
            });
        } else {
            self.metadata_size_bytes += value_size;
            self.metadata.insert(key, value);
        }
    }
    self
}
```

---

### 2.5 Metrics Data Structure Growth - MEDIUM

**Location**: `src/framework.rs:24-31`

```rust
#[derive(Debug, Default)]
pub struct FrameworkMetrics {
    concepts_injected_total: AtomicU64,
    associations_created_total: AtomicU64,
    probes_total: AtomicU64,
    probe_latency_ms_total: AtomicU64,
    probe_latency_count: AtomicU64,
}
```

**Issue**: Metrics use atomic counters which can theoretically overflow after 18 quintillion operations. In practice, this is not a concern, but there's no mechanism to reset or cap metrics.

**Remediation**: Consider adding periodic metrics reset capability:
```rust
impl FrameworkMetrics {
    pub fn reset(&self) {
        self.concepts_injected_total.store(0, Ordering::Relaxed);
        // ... reset others
    }
}
```

---

## 3. Async Memory Issues

### 3.1 RwLock Hold Duration - MEDIUM

**Location**: `src/framework.rs:153-162`

```rust
pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
    self.validate_top_k(top_k)?;
    let start = std::time::Instant::now();
    let sing = self.singularity.read().await;  // Lock held during computation!
    let results = sing.find_similar(&query, top_k);  // CPU-intensive operation
    let elapsed_ms = start.elapsed().as_millis() as u64;
    self.metrics.observe_probe_latency_ms(elapsed_ms);
    Ok(results)
}
```

**Issue**: The `RwLock` read guard is held during the entire `find_similar` computation, which is CPU-intensive. While this doesn't cause memory leaks directly, it can cause:
1. Writer starvation if many readers hold locks
2. Unbounded growth of waiting tasks in high-concurrency scenarios

**Remediation**:
```rust
pub async fn probe(&self, query: HVec10240, top_k: usize) -> Result<Vec<(String, f32)>> {
    self.validate_top_k(top_k)?;
    let start = std::time::Instant::now();
    
    // Clone data needed for computation, release lock
    let concepts = {
        let sing = self.singularity.read().await;
        sing.all_concepts()  // Clone to release lock quickly
    };
    
    // Perform computation outside lock
    let results = compute_similarity(&concepts, &query, top_k);
    
    let elapsed_ms = start.elapsed().as_millis() as u64;
    self.metrics.observe_probe_latency_ms(elapsed_ms);
    Ok(results)
}
```

---

### 3.2 Background Task Accumulation - MEDIUM

**Location**: `src/persistence.rs` (multiple locations)

**Issue**: Each async persistence operation acquires a semaphore permit and may spawn implicit background tasks through libsql. Under high load, if operations complete slower than they are queued, memory usage grows from:
1. Queued futures
2. Pending database operations
3. Backpressure accumulation

**Remediation**: Add explicit backpressure handling:
```rust
pub struct Persistence {
    // ... existing fields ...
    pending_operations: Arc<AtomicUsize>,
    max_pending_operations: usize,
}

impl Persistence {
    async fn check_backpressure(&self) -> Result<()> {
        let pending = self.pending_operations.load(Ordering::Relaxed);
        if pending > self.max_pending_operations {
            return Err(MemoryError::Database(
                "Too many pending operations".to_string()
            ));
        }
        Ok(())
    }
}
```

---

### 3.3 Stream Buffering Without Limits - MEDIUM

**Location**: `src/persistence_ops.rs:65-111`

```rust
pub async fn get_concept_history(&self, id: &str, limit: usize) -> Result<Vec<ConceptVersion>> {
    // ...
    let mut history = Vec::new();
    while let Some(row) = rows.next().await? {
        // ...
        history.push(ConceptVersion { ... });  // No memory limit on history!
    }
    Ok(history)
}
```

**Issue**: The `limit` parameter is passed to SQL, but there's no hard cap on memory allocation. Large version records with big metadata could still cause memory pressure.

**Remediation**:
```rust
const MAX_HISTORY_LIMIT: usize = 10_000;
const MAX_VERSION_SIZE_BYTES: usize = 1_000_000; // 1MB per version

pub async fn get_concept_history(&self, id: &str, limit: usize) -> Result<Vec<ConceptVersion>> {
    let limit = limit.min(MAX_HISTORY_LIMIT);
    // ... query with limit ...
    
    let mut total_size = 0usize;
    while let Some(row) = rows.next().await? {
        let metadata_json: String = row.get(3)?;
        total_size += metadata_json.len();
        
        if total_size > MAX_HISTORY_LIMIT * MAX_VERSION_SIZE_BYTES {
            return Err(MemoryError::InvalidInput {
                field: "history".to_string(),
                reason: "History exceeds memory limit".to_string(),
            });
        }
        // ...
    }
}
```

---

## 4. WASM Memory

### 4.1 JS-Rust Boundary Leaks - MEDIUM

**Location**: `src/wasm.rs:52-61`

```rust
let array = Array::new();
for (id, score) in results {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"id".into(), &id.into())
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
    js_sys::Reflect::set(&obj, &"score".into(), &score.into())
        .map_err(|_| JsValue::from_str("failed to set JS property"))?;
    array.push(&obj);
}
```

**Issue**: JavaScript objects created via `wasm-bindgen` are managed by JS GC. However, if the Rust side holds references (through `Closure`, `JsValue` in structs), cycles can prevent cleanup. The current code doesn't hold references, but large result sets create many JS objects.

**Remediation**: Add result set size limits for WASM:
```rust
#[wasm_bindgen]
impl WasmFramework {
    const MAX_WASM_RESULTS: usize = 1_000;
    
    pub async fn probe(&self, vector: &[u8], top_k: usize) -> Result<Array, JsValue> {
        if top_k > Self::MAX_WASM_RESULTS {
            return Err(JsValue::from_str(
                &format!("top_k exceeds WASM limit of {}", Self::MAX_WASM_RESULTS)
            ));
        }
        // ... rest of method
    }
}
```

---

### 4.2 Uint8Array Handling - LOW

**Location**: `src/wasm.rs:124-129`

```rust
let vector_bytes = vectors
    .get(i)
    .dyn_into::<Uint8Array>()
    .map_err(|_| JsValue::from_str("vector must be Uint8Array"))?
    .to_vec();  // Copies data - memory temporarily doubled
```

**Issue**: `to_vec()` creates a copy of the Uint8Array data. For large batch operations, this temporarily doubles memory usage.

**Status**: ACCEPTABLE - Temporary doubling is necessary for Rust ownership, but document this for users.

---

### 4.3 wasm-bindgen Memory Handling - LOW

**Location**: `src/wasm.rs:14-17`

```rust
#[wasm_bindgen]
pub struct WasmFramework {
    framework: ChaoticSemanticFramework,
}
```

**Analysis**: The `WasmFramework` struct holds a `ChaoticSemanticFramework` by value, not by reference. This is correct for WASM memory isolation. The framework will be dropped when the JS wrapper is GC'd.

**Status**: CLEAN - Correct implementation.

---

## 5. Cache Memory

### 5.1 LRU Cache Implementation Review - MEDIUM

**Location**: `src/singularity.rs:56-107`

```rust
#[derive(Debug, Default)]
struct QueryCache {
    capacity: usize,
    order: VecDeque<u64>,
    results: HashMap<u64, Arc<[(String, f32)]>>,
}
```

**Issue Analysis**:
1. **Capacity enforcement**: Correctly enforced at line 92-96
2. **Eviction policy**: LRU is correctly implemented
3. **Memory per entry**: No limit on result set size stored in cache

**Risk**: Cached entries with large `top_k` values (up to `max_cached_top_k = 100`) can consume significant memory per entry.

**Remediation**: Add entry size limit:
```rust
impl QueryCache {
    const MAX_RESULT_SIZE: usize = 10_000; // Max items per cached result
    
    fn put(&mut self, key: u64, value: Arc<[(String, f32)]>) -> bool {
        // Reject oversized results
        if value.len() > Self::MAX_RESULT_SIZE {
            return false;
        }
        // ... rest of implementation
    }
}
```

---

### 5.2 Cache Invalidation - MEDIUM

**Location**: `src/singularity.rs:411-415`

```rust
fn invalidate_cache(&self) {
    if let Ok(mut cache) = self.query_cache.lock() {
        cache.clear();
    }
}
```

**Issue**: Cache is invalidated on ANY modification (inject, delete, associate). This is conservative but correct. However, if the lock is poisoned, the cache is not cleared.

**Remediation**:
```rust
fn invalidate_cache(&self) {
    match self.query_cache.lock() {
        Ok(mut cache) => cache.clear(),
        Err(poisoned) => {
            // Even if poisoned, try to clear
            let mut cache = poisoned.into_inner();
            cache.clear();
        }
    }
}
```

---

### 5.3 Cache Metrics Memory - LOW

**Location**: `src/singularity.rs:109-131`

```rust
#[derive(Debug, Default)]
struct CacheMetrics {
    hits_total: AtomicU64,
    misses_total: AtomicU64,
    evictions_total: AtomicU64,
}
```

**Analysis**: Atomic counters use fixed memory. No unbounded growth possible.

**Status**: CLEAN

---

## 6. Resource Limits

### 6.1 Configurable Memory Limits Summary

The following limits exist but are not consistently enforced:

| Limit | Default | Enforced | Location |
|-------|---------|----------|----------|
| `max_concepts` | None | Soft | `SingularityConfig` |
| `max_associations_per_concept` | None | Soft | `SingularityConfig` |
| `concept_cache_size` | 128 | Hard | `QueryCache` |
| `max_cached_top_k` | 100 | Hard | `find_similar_cached` |
| `version_retention` | 10 | Soft | `Persistence` |
| `connection_pool_size` | 10 | Hard | `Persistence::new_turso_with_pool` |
| `max_probe_top_k` | 10,000 | Hard | `FrameworkConfig` |
| `max_metadata_bytes` | None | Soft | `FrameworkConfig` |

**Recommendation**: Add a `MemoryLimits` configuration struct:

```rust
// In src/framework_builder.rs
#[derive(Clone, Debug)]
pub struct MemoryLimits {
    /// Maximum memory for concept storage (approximate)
    pub max_concept_memory_bytes: usize,
    /// Maximum memory for association storage (approximate)  
    pub max_association_memory_bytes: usize,
    /// Maximum memory for query cache
    pub max_cache_memory_bytes: usize,
    /// Maximum total framework memory (hard ceiling)
    pub max_total_memory_bytes: usize,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_concept_memory_bytes: 100 * 1024 * 1024,      // 100 MB
            max_association_memory_bytes: 50 * 1024 * 1024,   // 50 MB
            max_cache_memory_bytes: 10 * 1024 * 1024,         // 10 MB
            max_total_memory_bytes: 500 * 1024 * 1024,        // 500 MB
        }
    }
}
```

---

### 6.2 Version Retention Configuration Gap - HIGH

**Location**: `src/persistence.rs:21-22`

```rust
pub struct Persistence {
    // ...
    pub(crate) version_retention: usize,  // Field exists but no setter!
}
```

**Issue**: The `version_retention` field is hardcoded to 10 in constructors but should be configurable.

**Remediation**:
```rust
// Add to FrameworkBuilder
pub fn with_version_retention(mut self, retention: usize) -> Self {
    self.version_retention = retention.clamp(1, 1000);
    self
}
```

---

### 6.3 Pool Size Limits - MEDIUM

**Location**: `src/persistence.rs:56-70`

```rust
pub async fn new_turso_with_pool(url: &str, token: &str, pool_size: usize) -> Result<Self> {
    // ...
    remote_limit: Some(Arc::new(Semaphore::new(pool_size.max(1)))),  // Only enforces >= 1
    // ...
}
```

**Issue**: No upper bound on pool size. Very large values could exhaust file descriptors or memory.

**Remediation**:
```rust
const MAX_POOL_SIZE: usize = 100;

pub async fn new_turso_with_pool(url: &str, token: &str, pool_size: usize) -> Result<Self> {
    let pool_size = pool_size.clamp(1, MAX_POOL_SIZE);
    // ...
}
```

---

## Remediation Roadmap

### Phase 1: Critical Fixes (Immediate)

1. **CRITICAL**: Add hard limit enforcement for version history (src/persistence.rs:450)
2. **HIGH**: Set default limits for concept storage (src/singularity.rs:386, src/framework_builder.rs:41)
3. **HIGH**: Set default limits for association storage (src/singularity.rs:304)

### Phase 2: High Priority (Week 1)

4. **HIGH**: Add `version_retention` setter to FrameworkBuilder
5. **HIGH**: Add metadata size limits to ConceptBuilder (src/concept_builder.rs:55)
6. **MEDIUM**: Add WASM-specific result limits (src/wasm.rs:42)

### Phase 3: Medium Priority (Week 2)

7. **MEDIUM**: Optimize RwLock hold duration in probe operations (src/framework.rs:153)
8. **MEDIUM**: Add backpressure handling for persistence operations
9. **MEDIUM**: Add pool size upper bounds (src/persistence.rs:56)
10. **MEDIUM**: Add cache entry size limits (src/singularity.rs:81)
11. **MEDIUM**: Fix lock poisoning in cache invalidation (src/singularity.rs:411)

### Phase 4: Low Priority (Week 3)

12. **LOW**: Add metrics reset capability (src/framework.rs:24)
13. **LOW**: Document temporary memory doubling in WASM batch operations
14. **LOW**: Document RAII semaphore behavior

---

## Testing Recommendations

1. **Memory Stress Test**: Create test that injects 1M+ concepts with large metadata
2. **Version History Test**: Verify version pruning under rapid update cycles
3. **WASM Memory Test**: Test large batch operations in browser environment
4. **Cache Eviction Test**: Verify LRU behavior and memory bounds
5. **Concurrent Load Test**: Test memory stability under high concurrent probe load

---

## GOAP Action Plan

### State Changes Required

| Current State | Desired State | Actions |
|--------------|---------------|---------|
| Unbounded concept growth | Bounded with safe defaults | Set DEFAULT_MAX_CONCEPTS = 100_000 |
| Unbounded association growth | Bounded with safe defaults | Set DEFAULT_MAX_ASSOCIATIONS = 10_000 |
| Fixed version_retention=10 | Configurable with hard ceiling | Add builder method + MAX_VERSION_RETENTION |
| No metadata size limit at builder | Builder-level validation | Add metadata_size_bytes tracking |
| Long RwLock holds | Minimal lock duration | Clone data before computation |
| No WASM result limits | WASM-specific limits | Add MAX_WASM_RESULTS constant |

### Preconditions

- All changes maintain backward compatibility
- Default limits must be configurable (not hardcoded)
- WASM compatibility must be preserved
- Performance benchmarks must not regress

---

## Conclusion

The codebase demonstrates good memory safety practices overall, but lacks defensive limits for long-running production use. The most critical issues are unbounded growth paths for concepts, associations, and version history. Implementing the recommended defaults and hard ceilings will make the system suitable for production deployments with predictable memory behavior.

**Risk Mitigation Priority**: 
1. Concept/Association limits (prevent OOM)
2. Version retention hard ceiling (prevent database bloat)
3. Cache size enforcement (prevent query cache abuse)
4. WASM limits (prevent browser tab crashes)
5. Async backpressure (prevent queue buildup)

**Estimated Implementation Effort**: 2-3 developer days for Phases 1-3.

---

*Report generated by Memory Leak Prevention Specialist (Swarm Groups B & D)*  
*Analysis scope: All source files in src/*  
*Methodology: Static code analysis + architectural review*
