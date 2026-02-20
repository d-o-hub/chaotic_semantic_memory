# ADR-0044: Memory Limits and Resource Governance

## Status

Proposed

## Context

The specialist analysis swarm identified multiple risks of unbounded memory growth in chaotic_semantic_memory:

1. **Unbounded Concept Storage** - `max_concepts` defaults to `None`, allowing unlimited memory growth
2. **Unbounded Association Storage** - No limit on associations per concept  
3. **Version History Without Hard Ceiling** - Version retention is configurable but not enforced at the database level
4. **Metadata Size Unvalidated** - No size limits during concept building
5. **Cache Memory Risk** - Query cache has no size bounds

In production deployments, these issues could lead to:
- OOM (Out of Memory) crashes
- Unpredictable memory usage
- Resource exhaustion attacks
- Difficult capacity planning

## Decision

Implement comprehensive memory governance with configurable limits and safe defaults.

### 1. Concept Limits

```rust
// src/singularity.rs
pub const DEFAULT_MAX_CONCEPTS: usize = 100_000;
pub const HARD_MAX_CONCEPTS: usize = 10_000_000;

pub struct Singularity {
    max_concepts: Option<usize>, // None = use default
    // ...
}

impl Singularity {
    pub fn with_max_concepts(mut self, limit: usize) -> Self {
        self.max_concepts = Some(limit.min(HARD_MAX_CONCEPTS));
        self
    }
}
```

### 2. Association Limits

```rust
// src/singularity.rs
pub const DEFAULT_MAX_ASSOCIATIONS: usize = 1_000;
pub const HARD_MAX_ASSOCIATIONS: usize = 100_000;

pub struct Concept {
    max_associations: Option<usize>,
    // ...
}
```

### 3. Version Retention

```rust
// src/persistence.rs
pub const DEFAULT_VERSION_RETENTION: usize = 10;
pub const MAX_VERSION_RETENTION: usize = 100;
pub const HARD_VERSION_CEILING: i64 = 1000; // Absolute max per concept

pub struct Persistence {
    version_retention: usize,
    enforce_hard_ceiling: bool, // Always true
}

impl Persistence {
    pub fn with_version_retention(mut self, retention: usize) -> Self {
        self.version_retention = retention.clamp(1, MAX_VERSION_RETENTION);
        self
    }
    
    async fn cleanup_old_versions(&self, concept_id: &str) -> Result<()> {
        // Enforce hard ceiling regardless of retention setting
        let sql = r#"
            DELETE FROM concept_versions 
            WHERE concept_id = ?1 
            AND version NOT IN (
                SELECT version FROM concept_versions 
                WHERE concept_id = ?1 
                ORDER BY modified_at DESC 
                LIMIT ?2
            )
        "#;
        // ...
    }
}
```

### 4. Metadata Size Validation

```rust
// src/concept_builder.rs
pub const MAX_METADATA_SIZE_BYTES: usize = 64 * 1024; // 64KB

impl ConceptBuilder {
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Result<Self> {
        let size = serde_json::to_vec(&metadata)?.len();
        if size > MAX_METADATA_SIZE_BYTES {
            return Err(MemoryError::InvalidInput {
                field: "metadata".to_string(),
                reason: format!("metadata size {} exceeds limit of {} bytes", 
                    size, MAX_METADATA_SIZE_BYTES),
            });
        }
        self.metadata = metadata;
        Ok(self)
    }
}
```

### 5. Cache Size Limits

```rust
// src/singularity.rs
pub const DEFAULT_CACHE_SIZE: usize = 10_000;
pub const MAX_CACHE_SIZE: usize = 1_000_000;

pub struct Singularity {
    query_cache: Option<LruCache<QueryCacheKey, Vec<(String, f32)>>>,
}

impl Singularity {
    pub fn with_query_cache_size(mut self, size: usize) -> Self {
        let size = size.clamp(100, MAX_CACHE_SIZE);
        self.query_cache = Some(LruCache::new(NonZeroUsize::new(size).unwrap()));
        self
    }
}
```

### 6. Framework Builder Integration

```rust
// src/framework_builder.rs
#[derive(Debug, Clone)]
pub struct FrameworkConfig {
    pub max_concepts: Option<usize>,
    pub max_associations: Option<usize>,
    pub version_retention: usize,
    pub query_cache_size: usize,
    pub max_metadata_size: usize,
}

impl Default for FrameworkConfig {
    fn default() -> Self {
        Self {
            max_concepts: Some(DEFAULT_MAX_CONCEPTS),
            max_associations: Some(DEFAULT_MAX_ASSOCIATIONS),
            version_retention: DEFAULT_VERSION_RETENTION,
            query_cache_size: DEFAULT_CACHE_SIZE,
            max_metadata_size: MAX_METADATA_SIZE_BYTES,
        }
    }
}

impl FrameworkBuilder {
    pub fn with_max_concepts(mut self, limit: usize) -> Self {
        self.config.max_concepts = Some(limit.min(HARD_MAX_CONCEPTS));
        self
    }
    
    pub fn with_version_retention(mut self, retention: usize) -> Self {
        self.config.version_retention = retention.clamp(1, MAX_VERSION_RETENTION);
        self
    }
    
    pub fn with_query_cache_size(mut self, size: usize) -> Self {
        self.config.query_cache_size = size.clamp(100, MAX_CACHE_SIZE);
        self
    }
}
```

## Consequences

### Positive

- Predictable memory usage in production
- Protection against resource exhaustion attacks
- Clear capacity planning guidelines
- Graceful degradation when limits reached
- Safe defaults prevent accidental misconfiguration

### Negative

- Additional configuration complexity for users
- Hard limits may reject legitimate use cases (mitigated by high ceilings)
- Migration needed for existing deployments with large datasets
- Slight performance overhead for limit checking

### Migration Path

Existing deployments exceeding new defaults:

1. **Detection:** Add startup warning when concepts > default limit
2. **Configuration:** Document how to raise limits via FrameworkBuilder
3. **Monitoring:** Add metrics for limit utilization
4. **Emergency override:** Environment variable to temporarily disable limits

## Compliance

- [x] Configurable limits with safe defaults
- [x] Hard ceilings prevent abuse
- [x] Validation at all entry points
- [x] Clear error messages when limits exceeded
- [x] Metrics for limit utilization
- [ ] CLI flags for limit configuration
- [ ] Documentation for capacity planning

## References

- Analysis: `plans/handoffs/analysis_memory_leaks.md`
- Master Coordination: `plans/handoffs/MASTER_ANALYSIS_COORDINATION.md`
- GOAP State: `plans/GOAP_STATE.md` Phase 27
