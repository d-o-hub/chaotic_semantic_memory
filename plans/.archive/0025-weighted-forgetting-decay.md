# [ADR-0025] Weighted Forgetting (Association Decay)

## Status
Deferred (Post-1.0)

**Rationale**: Analysis Swarm Consensus (2026-02-17) determined this is a research/advanced feature not required for core 1.0 functionality. Biological memory modeling is valuable but adds complexity without blocking production use cases.

## Context and Problem Statement

Biological memory exhibits graceful degradation - memories fade gradually rather than disappearing instantly. Current implementation has:
- Permanent associations (until explicit deletion)
- FIFO eviction when max_associations_per_concept reached
- No semantic notion of "memory strength" over time

This does not model natural memory decay patterns useful for:
- Prioritizing recent experiences in AI agents
- Modeling human-like memory in conversational systems
- Automatically pruning weak associations without hard limits
- Creating "importance" hierarchies from access patterns

## Decision Drivers

1. **Gradual Decay**: Associations weaken continuously, not abruptly
2. **Access Refresh**: Reading an association strengthens it (refresh)
3. **Configurable Rate**: Different use cases need different decay speeds
4. **Efficient Calculation**: Decay calculation must be O(1) per access
5. **Deterministic**: Same initial strength + time = same current strength
6. **Composable**: Works with existing association strength concept

## Considered Options

### Option 1: Access-Count Based
Strength based on number of accesses.

```rust
pub struct Association {
    base_strength: f32,
    access_count: u32,
}
```

**Pros:** Simple, no time dependency  
**Cons:** No time dimension, older but frequently-used associations treated same as new

### Option 2: Time-Based Decay (Chosen)
Strength decays exponentially with time since last access.

```rust
strength_t = strength_0 * exp(-lambda * delta_t)
where lambda = ln(2) / halflife
```

**Pros:** Natural biological model, configurable timescales, predictable  
**Cons:** Requires timestamp storage, clock dependency

### Option 3: Combined Decay + Access
Decay over time, but access partially restores strength.

**Pros:** Most biologically accurate  
**Cons:** More complex tuning, potential for runaway strengthening

## Decision Outcome

Chosen: **Option 2 - Exponential Decay with Configurable Halflife**

### Design

#### 1. Data Model

```rust
// In singularity.rs - Association storage
pub struct AssociationMeta {
    pub strength: f32,           // Current (calculated) strength
    pub last_accessed: u64,      // Unix timestamp
    pub initial_strength: f32,   // Strength at creation/last refresh
}

// Stored in associations HashMap
associations: HashMap<String, HashMap<String, AssociationMeta>>,
```

#### 2. Decay Formula

```rust
impl AssociationMeta {
    fn current_strength(&self, 
        now: u64, 
        halflife_seconds: f32
    ) -> f32 {
        if halflife_seconds <= 0.0 {
            return self.strength;
        }
        let delta_t = (now - self.last_accessed) as f32;
        let lambda = 0.693_147_2 / halflife_seconds;  // ln(2)
        self.initial_strength * (-lambda * delta_t).exp()
    }
}
```

#### 3. Configuration

```rust
pub struct FrameworkConfig {
    // ... existing ...
    pub association_decay_halflife_secs: Option<u64>,  // None = no decay
    pub decay_refresh_on_read: bool,                    // Access refreshes strength (default: true)
}
```

#### 4. API Changes

```rust
impl Singularity {
    /// Get associations with decay applied
    pub fn get_associations(&mut self, 
        id: &str, 
        now: u64
    ) -> Vec<(String, f32)> {
        // Calculate current strength for each association
        // If refresh_on_read, update last_accessed and initial_strength
    }
    
    /// Manually refresh an association (strengthen it)
    pub fn refresh_association(
        &mut self,
        from: &str,
        to: &str,
    ) -> Result<()>;
}
```

#### 5. Persistence

```sql
-- Add to associations table
ALTER TABLE associations ADD COLUMN last_accessed INTEGER DEFAULT (unixepoch());
ALTER TABLE associations ADD COLUMN initial_strength REAL DEFAULT strength;
```

On load, associations retain their metadata and continue decaying from loaded state.

### Positive Consequences

1. **Natural Memory Model**: Mimics biological forgetting curves
2. **Automatic Pruning**: Weak associations naturally fade without hard limits
3. **Temporal Reasoning**: Recent associations rank higher in search
4. **Zero Config**: Works out of box with sensible defaults
5. **Deterministic**: Same inputs always produce same outputs

### Negative Consequences

1. **Computation Cost**: exp() calculation on every read
2. **Storage Increase**: +16 bytes per association (last_accessed + initial_strength)
3. **Clock Dependency**: Requires system clock for decay calculation
4. **Tuning Complexity**: Finding right halflife requires experimentation

### Mitigations

1. **Performance**: exp() is fast on modern CPUs; cache current strength
2. **Storage**: Only store if decay enabled; use defaults for old data
3. **Clock**: Monotonic clock not required; wall clock sufficient
4. **Tuning**: Provide presets (short_term: 1 hour, long_term: 30 days)

## Implementation Notes

### Decay Presets

```rust
pub enum DecayPreset {
    Ephemeral,    // 5 minute halflife - very short term
    ShortTerm,    // 1 hour halflife - session memory
    MediumTerm,   // 1 day halflife - daily context
    LongTerm,     // 30 day halflife - persistent knowledge
    Permanent,    // No decay
}

impl DecayPreset {
    pub fn halflife_secs(&self) -> Option<u64> {
        match self {
            Ephemeral => Some(300),
            ShortTerm => Some(3600),
            MediumTerm => Some(86400),
            LongTerm => Some(2_592_000),
            Permanent => None,
        }
    }
}
```

### Integration with Query Cache

The query cache (existing) stores similarity results. When decay is enabled:
- Cache key must include current time bucket (e.g., minute-level)
- Or disable caching when decay is enabled
- Or accept slightly stale results (strengths may be slightly off)

**Decision:** Include time bucket in cache key for approximate freshness.

## Links

- Related ADRs:
  - ADR-0024: Concept Expiration (TTL) - complementary feature
  - ADR-0007: Similarity Search Optimization (query cache interaction)
