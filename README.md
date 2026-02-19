# chaotic_semantic_memory

[![CI](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml/badge.svg)](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/chaotic_semantic_memory.svg)](https://crates.io/crates/chaotic_semantic_memory)
[![docs.rs](https://img.shields.io/docsrs/chaotic_semantic_memory)](https://docs.rs/chaotic_semantic_memory)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`chaotic_semantic_memory` is a Rust crate for AI memory systems built on:
- 10240-bit hyperdimensional vectors
- chaotic echo-state reservoirs
- libSQL persistence

It targets both native and `wasm32` builds with explicit threading guards.

## Quick Links

| Resource | Link |
|----------|------|
| Documentation | [docs.rs/chaotic_semantic_memory](https://docs.rs/chaotic_semantic_memory) |
| Crates.io | [crates.io/crates/chaotic_semantic_memory](https://crates.io/crates/chaotic_semantic_memory) |
| Issues | [GitHub Issues](https://github.com/d-o-hub/chaotic_semantic_memory/issues) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

## Status

| Property | Value |
|----------|-------|
| Version | `0.1.0` |
| MSRV | Rust `1.85` |
| License | MIT |
| Targets | Native, `wasm32-unknown-unknown` |

## Installation

```bash
cargo add chaotic_semantic_memory
```

Enable WASM bindings when needed:

```toml
[dependencies]
chaotic_semantic_memory = { version = "0.1.0", features = ["wasm"] }
```

## Core Components

- `hyperdim`: binary hypervector math (`HVec10240`) and similarity operations
- `reservoir`: sparse chaotic reservoir dynamics with spectral radius controls
- `singularity`: concept graph, associations, retrieval, and memory limits
- `framework`: high-level async orchestration API
- `persistence`: libSQL-backed storage (native only)
- `wasm`: JS-facing bindings for browser/runtime integration
- `cli`: Command-line interface (`csm` binary)

## CLI Usage

The `csm` binary provides command-line access:

```bash
# Inject a concept
csm inject my-concept --database memory.db

# Find similar concepts
csm probe my-concept -k 10 --database memory.db

# Create associations
csm associate source-concept target-concept --strength 0.8

# Export memory state
csm export --output backup.json

# Import memory state
csm import backup.json --merge

# Generate shell completions
csm completions bash > ~/.local/share/bash-completion/completions/csm
```

### CLI Commands

| Command | Description |
|---------|-------------|
| `inject` | Inject a new concept with a random or provided vector |
| `probe` | Find similar concepts by concept ID |
| `associate` | Create an association between two concepts |
| `export` | Export memory state to JSON or binary |
| `import` | Import memory state from file |
| `version` | Show version information |
| `completions` | Generate shell completions |

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
See `examples/basic_in_memory.rs` for the minimal in-memory workflow.

## Configuration

`ChaoticSemanticFramework::builder()` exposes runtime tuning knobs.

| Parameter | Default | Valid Range | Effect |
|---|---:|---|---|
| `reservoir_size` | `50_000` | `> 0` | Reservoir capacity and memory footprint |
| `reservoir_input_size` | `10_240` | `> 0` | Width of each sequence step |
| `chaos_strength` | `0.1` | `0.0..=1.0` (recommended) | Noise amplitude in chaotic updates |
| `enable_persistence` | `true` | boolean | Enables libSQL persistence setup |
| `max_concepts` | `None` | optional positive | Evicts oldest concepts when reached |
| `max_associations_per_concept` | `None` | optional positive | Keeps strongest associations only |
| `connection_pool_size` | `10` | `>= 1` | Turso/libSQL remote pool size |
| `max_probe_top_k` | `10_000` | `>= 1` | Input guard for `probe` and batch probes |
| `max_metadata_bytes` | `None` | optional positive | Metadata payload size guard |
| `concept_cache_size` | `1_000` | `>= 1` | Similarity query cache capacity |

### Tuning Guide

- Small workloads: disable persistence and use `reservoir_size` around `10_240`.
- Mid-sized workloads: keep defaults and set `max_concepts` to enforce memory ceilings.
- Large workloads: keep persistence enabled, increase `connection_pool_size`, and tune `max_probe_top_k` to practical limits.

## API Patterns

In-memory flow:

```rust
let framework = ChaoticSemanticFramework::builder()
    .without_persistence()
    .build()
    .await?;
```

Persistent flow:

```rust
let framework = ChaoticSemanticFramework::builder()
    .with_local_db("memory.db")
    .build()
    .await?;
```

Batch APIs for bulk workloads:

```rust
framework.inject_concepts(&concepts).await?;
framework.associate_many(&edges).await?;
let hits = framework.probe_batch(&queries, 10).await?;
```

Load semantics:

- `load_replace()`: clear in-memory state, then load persisted data.
- `load_merge()`: merge persisted state into current in-memory state.

## WASM Build

```bash
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown
```

Notes:
- WASM threading-sensitive paths are guarded with `#[cfg(not(target_arch = "wasm32"))]`.
- Persistence is intentionally unavailable on `wasm32` in this crate build.
- WASM parity APIs include `processSequence`, `exportToBytes`, and `importFromBytes`.

## Development Gates

```bash
cargo check --quiet
cargo test --all-features --quiet
cargo fmt --check --quiet
cargo clippy --quiet -- -D warnings
```

LOC policy: each source file in `src/` must stay at or below 500 lines.

## Mutation Testing

Install cargo-mutants once:

```bash
cargo install cargo-mutants
```

Run profiles:

```bash
scripts/mutation_test.sh fast
scripts/mutation_test.sh full
```

Reports are written under `progress/mutation/`.

## Benchmark Gates

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
```

Primary perf gate: `reservoir_step_50k < 100us`.

## Security

### Reporting Vulnerabilities

Please report security vulnerabilities privately via [GitHub Security Advisories](https://github.com/d-o-hub/chaotic_semantic_memory/security/advisories/new).

Do not file public issues for security bugs.

### Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

### Security Features

- No hardcoded secrets or credentials in source code
- Input validation on all public APIs
- Memory limits enforced via `max_concepts`, `max_metadata_bytes`, and `max_probe_top_k`
- WASM build excludes persistence layer (no filesystem access)

## License

MIT
