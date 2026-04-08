# Text Encoding

`TextEncoder` maps text to `HVec10240` for direct use with concept injection and probing.

## Why It Exists

- Deterministic text-to-vector conversion for repeatable indexing/querying
- No external model dependency
- Works in native Rust and WASM builds

## Basic Usage

```rust,no_run
use chaotic_semantic_memory::encoder::TextEncoder;

let encoder = TextEncoder::new();
let vector = encoder.encode("rust async memory");
```

## N-gram Encoding

N-grams improve local phrase sensitivity:

```rust,no_run
use chaotic_semantic_memory::encoder::TextEncoder;

let encoder = TextEncoder::new();
let vector = encoder.encode_with_ngrams("chaotic semantic memory", 3);
```

## Framework Convenience APIs

```rust,no_run
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    framework.inject_text("doc-1", "Rust uses ownership for memory safety").await?;
    let hits = framework.probe_text("memory safety in rust", 5).await?;
    assert!(!hits.is_empty());
    Ok(())
}
```

## Semantic Similarity Alternative

`TextEncoder` produces vectors for **lexical similarity** (same tokens, same order).
For **semantic similarity** (synonyms, paraphrases), you have two options:

### Option 1: External Embedding Model

Use `sentence-transformers` or similar, then inject the resulting vector:

```rust,no_run
let embedding: HVec10240 = my_model.encode("an overview of echo-state networks");
framework.inject_concept("doc-2", embedding).await?;
```

### Option 2: Turso Native Vectors

This crate uses libSQL (local SQLite or remote Turso) for persistence. You can
add Turso's native `F32_BLOB` vector tables alongside the crate's HDC storage:

```rust,no_run
use libsql::Builder;

// Connect to the same database this crate uses
let db = Builder::new_local("csm_memory.db").build().await?;
let conn = db.connect()?;

conn.execute_batch("
    CREATE TABLE IF NOT EXISTS semantic_vectors (
        id TEXT PRIMARY KEY,
        embedding F32_BLOB(384)
    );
    CREATE INDEX IF NOT EXISTS semantic_idx ON semantic_vectors(
        libsql_vector_idx(embedding, 'metric=cosine')
    );
").await?;
```

Both HDC concepts and semantic vectors live in the **same database**. The crate
manages `concepts` and `associations` tables, while you manage `semantic_vectors`
for float-vector similarity search via `vector_top_k()`.

## Hashing Notes

- Default hashing is FNV-1a for stable cross-platform behavior.
- Switching hash algorithms changes produced vectors for the same text.
- If you persist encoder-generated vectors, re-encoding policy should be part of migration planning.
