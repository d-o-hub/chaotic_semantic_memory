# [ADR-0024] Concept Expiration (TTL)

## Status
Deferred (Post-1.0)

**Rationale**: Analysis Swarm Consensus (2026-02-17) determined this feature, while valuable, is not required for 1.0 release. Current system is production-ready without TTL. Scheduled for 2.0 or based on user demand.

## Context and Problem Statement

Production deployments of chaotic_semantic_memory need automatic cleanup mechanisms for:
- Session-based temporary concepts that should not persist indefinitely
- Time-sensitive data that becomes irrelevant after a period
- Memory-constrained environments where old data must be evicted
- Compliance requirements for data retention limits

Currently, concepts exist until explicitly deleted or until max_concepts limit triggers FIFO eviction. Neither approach provides time-based lifecycle management.

## Decision Drivers

1. **Automatic Cleanup**: Must work without explicit user intervention
2. **Configurable Per-Concept**: Different concepts may have different lifetimes
3. **WASM Compatible**: Must work in browser environments without persistence
4. **Query-Time Filtering**: Expired concepts should not appear in search results
5. **Efficient Cleanup**: Should not require full table scans
6. **Backward Compatible**: Existing code continues to work unchanged

## Considered Options

### Option 1: Global TTL Policy
A single TTL applies to all concepts in the framework.

```rust
pub struct FrameworkConfig {
    pub global_ttl_seconds: Option<u64>,
}
```

**Pros:**
- Simple implementation
- Single index on expires_at
- Predictable behavior

**Cons:**
- Inflexible - all concepts have same lifetime
- Cannot have permanent + temporary concepts together

### Option 2: Per-Concept TTL (Chosen)
Each concept carries its own expiration timestamp.

```rust
pub struct Concept {
    // ... existing fields ...
    pub expires_at: Option<u64>,  // Unix timestamp, None = never expires
}
```

**Pros:**
- Flexible - mix permanent and temporary concepts
- Natural fit for session-based data
- Supports compliance use cases (different retention per data type)

**Cons:**
- Slightly more complex API
- More storage per concept (8 bytes for timestamp)

### Option 3: TTL on Associations Only
Keep concepts permanent, but associations expire.

**Pros:**
- Concepts remain stable identifiers
- Natural for "fading memory" semantics

**Cons:**
- Does not solve storage pressure from concept vectors
- Does not address compliance data deletion requirements

## Decision Outcome

Chosen: **Option 2 - Per-Concept TTL with optional global default**

### Implementation Design

#### 1. Data Model Changes

```rust
// In singularity.rs - Concept struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub vector: HVec10240,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub modified_at: u64,
    pub expires_at: Option<u64>,  // NEW: None = never expires
}

// In FrameworkConfig
pub struct FrameworkConfig {
    // ... existing fields ...
    pub default_ttl_seconds: Option<u64>,  // Applied when no explicit TTL given
}
```

#### 2. Database Schema Migration

```sql
-- Migration for existing databases
ALTER TABLE concepts ADD COLUMN expires_at INTEGER;
CREATE INDEX idx_concepts_expires ON concepts(expires_at) 
    WHERE expires_at IS NOT NULL;
```

#### 3. API Additions

```rust
impl ChaoticSemanticFramework {
    /// Inject concept with explicit TTL
    pub async fn inject_concept_with_ttl(
        &self,
        id: impl Into<String>,
        vector: HVec10240,
        ttl_seconds: u64,
    ) -> Result<()>;

    /// Get concepts that will expire within the given window
    pub async fn get_expiring_concepts(
        &self,
        within_seconds: u64,
    ) -> Result<Vec<String>>;

    /// Remove expired concepts (returns count removed)
    pub async fn cleanup_expired(&self) -> Result<usize>;
    
    /// Extend TTL for an existing concept
    pub async fn extend_ttl(
        &self,
        id: &str,
        additional_seconds: u64,
    ) -> Result<()>;
}
```

#### 4. WASM API Parity

```rust
#[wasm_bindgen]
impl WasmFramework {
    pub async fn inject_concept_with_ttl(
        &self,
        id: String,
        vector: &[u8],
        ttl_seconds: u64,
    ) -> Result<(), JsValue>;

    /// Cleanup triggered automatically or manually
    pub async fn cleanup_expired(&self) -> Result<u32, JsValue>;
}
```

#### 5. Expiration Enforcement

**Query-Time Filtering:**
```rust
pub fn find_similar(&self, query: &HVec10240, top_k: usize) -> Vec<(String, f32)> {
    let now = unix_now_secs();
    // Filter expired concepts before similarity calculation
    let active_concepts: Vec<_> = self.concepts.values()
        .filter(|c| c.expires_at.map(|t| t > now).unwrap_or(true))
        .collect();
    // ... rest of search
}
```

**Background Cleanup:**
- Cleanup triggered periodically (configurable interval)
- Also triggered on query if expired concepts detected
- In WASM: cleanup on every Nth operation to avoid blocking

#### 6. Configuration

```rust
pub struct FrameworkConfig {
    // ... existing ...
    pub default_ttl_seconds: Option<u64>,          // Default TTL for new concepts
    pub cleanup_interval_secs: u64,                 // Background cleanup frequency (default: 300)
    pub cleanup_on_query: bool,                     // Check expiration during queries (default: true)
}
```

### Positive Consequences

1. **Automatic Resource Management**: Old data cleans itself up
2. **Compliance Ready**: Supports GDPR/CCPA retention requirements
3. **Session Support**: Natural fit for temporary session data
4. **WASM Compatible**: Works fully in browser without persistence
5. **Backward Compatible**: Existing `inject_concept` uses `None` TTL
6. **Flexible**: Mix permanent and ephemeral data in same framework

### Negative Consequences

1. **Storage Overhead**: 8 bytes per concept for expires_at field
2. **Query Overhead**: Filter check on every similarity search
3. **Clock Dependency**: Requires reliable system clock
4. **Migration Required**: Existing databases need schema update

### Mitigations

1. **Storage**: Optional field uses null in SQLite (minimal overhead)
2. **Query**: Index on expires_at + early filter before similarity calc
3. **Clock**: Monotonic clock not required - wall clock acceptable for TTL
4. **Migration**: Automatic via existing schema migration system (ADR-0021)

## Implementation Plan

### Phase 1: Core Implementation
1. Add `expires_at` to Concept struct
2. Add migration SQL for concepts table
3. Implement `inject_concept_with_ttl()`
4. Update `find_similar()` to filter expired
5. Implement `cleanup_expired()`

### Phase 2: WASM Parity
1. Expose `inject_concept_with_ttl` in wasm.rs
2. Expose `cleanup_expired` in wasm.rs
3. Add automatic cleanup trigger in WASM (every N ops)

### Phase 3: Configuration
1. Add TTL config options to FrameworkConfig
2. Add default TTL behavior
3. Add cleanup interval configuration

### Phase 4: Testing
1. Unit tests for expiration logic
2. Integration tests for cleanup
3. WASM tests for TTL behavior
4. Migration test from schema v2 -> v3

## LOC Budget

- singularity.rs: +20 lines (Concept struct, filter logic)
- framework.rs: +40 lines (new methods, config)
- framework_ops.rs: +30 lines (cleanup implementation)
- persistence.rs: +15 lines (schema migration, save/load)
- wasm.rs: +20 lines (WASM bindings)

**Total: ~125 lines** - Well within 500 LOC per file constraint

## Size Impact

- Binary increase: ~200-300 bytes (minimal)
- Memory per concept: +8 bytes (expires_at Option<u64>)
- WASM compatible: Yes, no additional dependencies

## Dependencies

No new dependencies required. Uses existing:
- serde for serialization
- libsql for persistence (native only)

## Links

- Related ADRs:
  - ADR-0021: Schema Migration Support (for database migration)
  - ADR-0017: Concept Versioning (similar pattern for metadata)
- Issues: None yet (proposed)
- PRs: None yet (proposed)
