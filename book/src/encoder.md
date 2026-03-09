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

## Hashing Notes

- Default hashing is FNV-1a for stable cross-platform behavior.
- Switching hash algorithms changes produced vectors for the same text.
- If you persist encoder-generated vectors, re-encoding policy should be part of migration planning.
