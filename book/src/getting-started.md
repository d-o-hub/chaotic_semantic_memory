# Getting Started

## Installation

Add to your `Cargo.toml`:

```bash
cargo add chaotic_semantic_memory
```

For WASM bindings:

```toml
[dependencies]
chaotic_semantic_memory = { version = "0.4", features = ["wasm"] }
```

## Requirements

- Rust 1.85+ (MSRV)
- For WASM: `wasm32-unknown-unknown` target

## Minimal Example

```rust,no_run
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create in-memory framework
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await?;

    // Inject concepts
    let cat = ConceptBuilder::new("cat".to_string()).build();
    let dog = ConceptBuilder::new("dog".to_string()).build();
    
    framework.inject_concept("cat".to_string(), cat.vector.clone()).await?;
    framework.inject_concept("dog".to_string(), dog.vector.clone()).await?;

    // Create association
    framework.associate("cat".to_string(), "dog".to_string(), 0.8).await?;

    // Query similar concepts
    let hits = framework.probe(cat.vector.clone(), 5).await?;
    for (id, similarity) in hits {
        println!("{}: {:.3}", id, similarity);
    }

    Ok(())
}
```

## With Persistence

```rust,no_run
use chaotic_semantic_memory::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let framework = ChaoticSemanticFramework::builder()
        .with_local_db("csm_memory.db")
        .build()
        .await?;

    // Data persists across restarts
    framework.load_replace().await?;
    
    Ok(())
}
```

## CLI Installation

The `csm` binary is included with default features:

```bash
cargo install chaotic_semantic_memory --features cli
csm --help
```
