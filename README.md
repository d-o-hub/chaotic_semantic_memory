# chaotic_semantic_memory

Production-oriented Rust crate for chaotic semantic memory with:
- singularity mode concept injection/probe,
- Turso-style persistence surface,
- object-store checkpoint hooks,
- wasm-compatible bindings

## Verify

```bash
cargo check
cargo test
cargo run --example singularity
TURSO_DATABASE_URL='file:/tmp/chaotic_semantic_memory_demo.db' TURSO_AUTH_TOKEN='local-token' cargo run --example turso_agent
cargo bench --bench benchmark
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```
