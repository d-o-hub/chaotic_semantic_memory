# Skill Memory API Reference

Complete API reference for the `skill-memory` system.

## Core Types

### `SkillMemory`

Main handle providing memory operations for a skill.

```rust
pub struct SkillMemory {
    framework: ChaoticSemanticFramework,
    namespace: String,
    config: MemoryConfig,
}
```

#### Methods

##### `initialize(skill_name: &str) -> Result<Self, MemoryError>`

Initialize memory for a skill. Reads configuration from `AGENTS.md`.

**Example:**
```rust
let memory = SkillMemory::initialize("adr-creation").await?;
```

##### `remember(operation: &str, context: &str, result: &str) -> Result<String, MemoryError>`

Store an operation with its context and result.

**Parameters:**
- `operation`: Identifier for the operation type (e.g., "refactor_module")
- `context`: Input parameters, file paths, state description
- `result`: Outcome, success/failure, metrics

**Returns:** Unique concept ID

**Example:**
```rust
let id = memory.remember(
    "architectural_decision",
    "ADR-0043: CSM Integration",
    "Approved: High value, low risk"
).await?;
// Returns: "skill::adr-creation::architectural_decision::1708432000"
```

##### `recall(query: &str, similarity_threshold: f32, top_k: usize) -> Result<Vec<MemoryEntry>, MemoryError>`

Find similar past operations using semantic similarity.

**Parameters:**
- `query`: Natural language or structured query
- `similarity_threshold`: Minimum similarity score (0.0-1.0)
- `top_k`: Maximum number of results to return

**Returns:** Vector of memory entries sorted by similarity

**Example:**
```rust
let memories = memory.recall(
    "CSM integration for skills",
    0.7,
    5
).await?;

for entry in memories {
    println!("{}: {:.2}", entry.operation, entry.similarity);
}
```

##### `associate(concept1: &str, concept2: &str, strength: f32) -> Result<(), MemoryError>`

Create a weighted association between two concepts.

**Parameters:**
- `concept1`: First concept ID
- `concept2`: Second concept ID
- `strength`: Association weight (0.0-1.0)

**Example:**
```rust
memory.associate(
    "error::E0495::abc123",
    "solution::lifetime::def456",
    0.95
).await?;
```

##### `related(concept_id: &str, min_strength: f32) -> Result<Vec<(MemoryEntry, f32)>, MemoryError>`

Get concepts associated with the given concept.

**Parameters:**
- `concept_id`: Concept to find relations for
- `min_strength`: Minimum association strength to include

**Returns:** Vector of (entry, strength) tuples

**Example:**
```rust
let related = memory.related("error::E0495::abc123", 0.8).await?;
for (entry, strength) in related {
    println!("{} (strength: {:.2})", entry.id, strength);
}
```

##### `text_to_hypervector(text: &str) -> Result<HVec10240, MemoryError>`

Convert text to a hypervector for custom operations.

**Parameters:**
- `text`: Input text to encode

**Returns:** 10240-bit hypervector

**Example:**
```rust
let query_vec = memory.text_to_hypervector(
    "authentication middleware"
).await?;
```

##### `persist() -> Result<(), MemoryError>`

Force immediate persistence to disk.

**Example:**
```rust
memory.persist().await?;
```

### `MemoryEntry`

Represents a retrieved memory.

```rust
pub struct MemoryEntry {
    pub id: String,                    // Unique concept ID
    pub operation: String,             // Operation type
    pub context: String,               // Input context
    pub result: String,                // Outcome
    pub timestamp: u64,                // Unix timestamp
    pub similarity: f32,               // Similarity to query (0.0-1.0)
    pub metadata: HashMap<String, Value>, // Full metadata
}
```

### `MemoryConfig`

Configuration for skill memory.

```rust
pub struct MemoryConfig {
    pub database: DatabaseConfig,
    pub namespaces: NamespaceConfig,
    pub persistence: PersistenceConfig,
    pub limits: LimitConfig,
}

pub struct DatabaseConfig {
    pub db_type: DatabaseType,  // Local | Global | Custom
    pub path: Option<PathBuf>,
}

pub struct NamespaceConfig {
    pub mode: NamespaceMode,    // PerSkill | Shared | Hybrid
}

pub struct PersistenceConfig {
    pub auto_save_interval: usize,
    pub auto_save_on_complete: bool,
    pub async_persistence: bool,
}

pub struct LimitConfig {
    pub max_concepts_per_skill: usize,
    pub max_associations_per_concept: usize,
    pub max_metadata_bytes: usize,
}
```

## Error Types

### `MemoryError`

```rust
pub enum MemoryError {
    Database(libsql::Error),
    Serialization(serde_json::Error),
    Validation(String),
    Configuration(String),
    Csm(MemoryError),  // From chaotic_semantic_memory
}
```

## Advanced Operations

### Custom Concept IDs

For operations needing specific IDs:

```rust
let custom_id = format!("skill::{}::pattern::{}",
    skill_name, pattern_hash);

// Use low-level framework API
memory.framework().inject_concept_with_metadata(
    &custom_id,
    vector,
    metadata
).await?;
```

### Batch Operations

For high-throughput scenarios:

```rust
use skill_memory::BatchMemory;

let batch = memory.batch();

for operation in operations {
    batch.remember(
        operation.name,
        operation.context,
        operation.result
    )?;
}

// Persist all at once
batch.commit().await?;
```

### Query Cache

Enable query result caching for repeated similar queries:

```rust
let memory = SkillMemory::initialize("my-skill")
    .with_query_cache(1000)  // Cache size
    .with_max_cached_top_k(50)  // Max top_k to cache
    .await?;
```

### Metadata Extensions

Store custom metadata beyond the default fields:

```rust
use skill_memory::ExtendedMemory;

let ext_memory = memory.with_extensions();

let id = ext_memory.remember_with_metadata(
    "code_review",
    "src/lib.rs",
    "Approved",
    hashmap! {
        "reviewer" => "alice",
        "lines_changed" => 42,
        "test_coverage" => 0.95
    }
).await?;
```

## Performance Considerations

### Recall Optimization

- Use appropriate `top_k` values (5-10 usually sufficient)
- Set `similarity_threshold` to filter early (0.6-0.8)
- Enable query caching for repeated queries
- Use batch operations for bulk inserts

### Memory Limits

Default limits prevent unbounded growth:
- 10,000 concepts per skill
- 50 associations per concept
- 64KB metadata per concept

When limits exceeded:
- LRU eviction for concepts
- Weakest-first eviction for associations

### Persistence Strategy

**Auto-save modes:**
- `auto_save_interval: 10` - Save every 10 operations
- `auto_save_on_complete: true` - Save after skill execution
- Manual `persist()` for critical points

**Trade-offs:**
- Frequent saves: Durability, but I/O overhead
- Batched saves: Performance, but risk of data loss

## Configuration Examples

### High-Durability Mode

For critical skills where data loss is unacceptable:

```yaml
memory:
  persistence:
    auto_save_interval: 1  # Save after every operation
    auto_save_on_complete: true
    async_persistence: false  # Sync for durability
```

### High-Performance Mode

For throughput-sensitive skills:

```yaml
memory:
  persistence:
    auto_save_interval: 100  # Batch 100 operations
    auto_save_on_complete: true
    async_persistence: true
  limits:
    max_concepts_per_skill: 50000  # Larger cache
```

### Shared Memory Mode

For cross-skill pattern learning:

```yaml
memory:
  database:
    type: global  # ~/.config/opencode/memory.db
  namespaces:
    mode: hybrid
    shared_prefix: "shared::"
```

## Integration with CSM Directly

For advanced use cases, access the underlying CSM framework:

```rust
let framework = memory.framework();

// Use any ChaoticSemanticFramework API
let concept = framework.get_concept(&id).await?;
let associations = framework.get_associations(&id).await?;
let temporal = framework.process_sequence(&inputs).await?;
```

See `chaotic_semantic_memory` crate documentation for full CSM API.
