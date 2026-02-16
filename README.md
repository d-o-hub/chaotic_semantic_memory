# chaotic_semantic_memory

`chaotic_semantic_memory` is a Rust crate for AI memory systems built on:
- 10240-bit hyperdimensional vectors
- chaotic echo-state reservoirs
- libSQL persistence

It targets both native and `wasm32` builds with explicit threading guards.

## Core Components

- `hyperdim`: binary hypervector math (`HVec10240`) and similarity operations
- `reservoir`: sparse chaotic reservoir dynamics with spectral radius controls
- `singularity`: concept graph, associations, retrieval, and memory limits
- `framework`: high-level async orchestration API
- `persistence`: libSQL-backed storage (native only)
- `wasm`: JS-facing bindings for browser/runtime integration

## Quick Start

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
    println!("hits: {}", hits.len());
    Ok(())
}
```

See `examples/proof_of_concept.rs` for an end-to-end flow.

## WASM Build

```bash
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown
```

Notes:
- WASM threading-sensitive paths are guarded with `#[cfg(not(target_arch = "wasm32"))]`.
- Persistence is intentionally unavailable on `wasm32` in this crate build.

## Development Gates

```bash
cargo check --quiet
cargo test --all-features --quiet
cargo fmt --check --quiet
cargo clippy --quiet -- -D warnings
```

LOC policy: each source file in `src/` must stay at or below 500 lines.

## Benchmark Gates

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```

Primary perf gate: `reservoir_step_50k < 100us`.

## License

MIT
