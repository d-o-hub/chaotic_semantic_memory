# Configuration

## Framework Builder

```rust
let framework = ChaoticSemanticFramework::builder()
    // Reservoir settings
    .with_reservoir_size(50_000)
    .with_reservoir_input_size(10_240)
    .with_chaos_strength(0.1)
    
    // Persistence settings
    .with_local_db("memory.db")
    .with_connection_pool_size(10)
    
    // Memory limits
    .with_max_concepts(1_000_000)
    .with_max_associations_per_concept(100)
    .with_max_metadata_bytes(4096)
    
    // Cache settings
    .with_concept_cache_size(1_000)
    .with_max_cached_top_k(100)
    
    // Query limits
    .with_max_probe_top_k(10_000)
    
    .build()
    .await?;
```

## Parameters

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `reservoir_size` | 50,000 | >0 | Reservoir node count |
| `reservoir_input_size` | 10,240 | >0 | Input dimension |
| `chaos_strength` | 0.1 | 0.0-1.0 | Noise amplitude |
| `connection_pool_size` | 10 | ≥1 | Remote DB pool |
| `max_concepts` | None | >0 | Evict oldest when reached |
| `max_associations_per_concept` | None | >0 | Keep strongest only |
| `max_metadata_bytes` | None | >0 | Metadata size limit |
| `concept_cache_size` | 1,000 | ≥1 | LRU cache capacity |
| `max_cached_top_k` | 100 | ≥1 | Bypass cache above this |
| `max_probe_top_k` | 10,000 | ≥1 | Query guard limit |

## Persistence Options

### No Persistence

```rust
.without_persistence()
```

In-memory only, no database.

### Local SQLite

```rust
.with_local_db("memory.db")
```

Local SQLite file.

### Remote Turso

```rust
.with_remote_db("libsql://...", auth_token)
```

Remote Turso/LibSQL database with authentication.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CSM_DATABASE_PATH` | Default database path |
| `CSM_LOG_LEVEL` | Log level (trace, debug, info, warn, error) |
| `CSM_RESERVOIR_SIZE` | Override reservoir size |
