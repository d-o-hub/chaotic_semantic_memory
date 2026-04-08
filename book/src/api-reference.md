# API Reference

## Prelude

Import common types:

```rust
use chaotic_semantic_memory::prelude::*;
```

This exports:
- `HVec10240`, `ChaoticSemanticFramework`, `FrameworkBuilder`
- `Concept`, `ConceptBuilder`, `MemoryError`, `Result`

## Core Types

### HVec10240

10240-bit binary hypervector.

```rust
let v1 = HVec10240::random();
let v2 = HVec10240::random();

// Operations
let bundled = HVec10240::bundle(&[v1, v2])?;
let bound = v1.bind(&v2);
let permuted = v1.permute(128);

// Similarity
let sim = v1.cosine_similarity(&v2);
```

### ChaoticSemanticFramework

Main orchestration type.

```rust
let framework = ChaoticSemanticFramework::builder()
    .with_reservoir_size(50_000)
    .with_chaos_strength(0.1)
    .with_local_db("csm_memory.db")
    .build()
    .await?;
```

### Concept

Stored concept with vector and metadata.

```rust
let concept = ConceptBuilder::new("my-id".to_string())
    .with_vector(HVec10240::random())
    .with_metadata("key", "value")
    .build()?;
```

## Async Operations

All framework operations are async:

```rust
// Single operations
framework.inject_concept(id, vector).await?;
framework.associate(from, to, strength).await?;
let hits = framework.probe(vector, 10).await?;

// Batch operations
framework.inject_concepts(&concepts).await?;
framework.associate_many(&edges).await?;
let results = framework.probe_batch(&vectors, 10).await?;
```

## Error Handling

All fallible APIs return `Result<T, MemoryError>`:

```rust
match framework.probe(vector, 10).await {
    Ok(hits) => println!("Found {} hits", hits.len()),
    Err(MemoryError::NotFound { entity, id }) => eprintln!("{} not found: {}", entity, id),
    Err(e) => eprintln!("Error: {}", e),
}
```
