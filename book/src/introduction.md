# Introduction

`chaotic_semantic_memory` is a Rust crate for AI memory systems built on:

- **10240-bit hyperdimensional vectors** - Dense binary representations for semantic similarity
- **Chaotic echo-state reservoirs** - Temporal sequence processing with emergent dynamics
- **libSQL persistence** - Durable storage with SQLite compatibility

## Why Chaotic Memory?

Traditional vector databases store static embeddings. Chaotic semantic memory adds:

1. **Temporal processing** - Sequences are compressed into single hypervectors
2. **Emergent dynamics** - Reservoir chaos enables associative recall
3. **Sub-symbolic reasoning** - Hypervector operations are neuro-symbolic

## Vector Options

This crate uses **Hyperdimensional Computing (HDC)** for fast lexical similarity.
For semantic similarity (synonyms, paraphrases), you can:

- **External embeddings**: Use `sentence-transformers` or similar, inject via `inject_concept()`
- **Turso native vectors**: Add `F32_BLOB` tables to the same libSQL/Turso database this crate uses
- **Hybrid approach**: HDC for lexical matching + Turso `vector_top_k()` for semantic search

See [Text Encoding](./encoder.md) for details.

## Features

- Native Rust with WASM target support
- SIMD-accelerated hypervector operations
- Sparse reservoir matrices for memory efficiency
- Async I/O with Tokio
- Connection pooling for remote databases
- Export/import for data migration

## Quick Example

```rust
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    let concept = ConceptBuilder::new("cat".to_string()).build();
    framework.inject_concept("cat".to_string(), concept.vector.clone()).await?;

    let hits = framework.probe(concept.vector.clone(), 5).await?;
    println!("Found {} similar concepts", hits.len());
    Ok(())
}
```
