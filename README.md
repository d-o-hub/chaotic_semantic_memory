# d.o. chaotic semantic memory

[![CI](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml/badge.svg)](https://github.com/d-o-hub/chaotic_semantic_memory/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/chaotic_semantic_memory.svg)](https://crates.io/crates/chaotic_semantic_memory)
[![docs.rs](https://img.shields.io/docsrs/chaotic_semantic_memory)](https://docs.rs/chaotic_semantic_memory)
[![npm](https://img.shields.io/npm/v/@d-o-hub/chaotic_semantic_memory)](https://www.npmjs.com/package/@d-o-hub/chaotic_semantic_memory)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

<img width="1456" height="720" alt="1000046937" src="https://github.com/user-attachments/assets/d698a304-9b36-4aad-a2d1-5668157a26f7" />

`chaotic_semantic_memory` is a Rust crate for AI memory systems built on
**Hyperdimensional Computing (HDC)** — not transformer embeddings:
- 10240-bit binary hypervectors with SIMD-accelerated operations
- chaotic echo-state reservoirs for temporal processing
- libSQL persistence (local SQLite or remote Turso)

It targets both native and `wasm32` builds with explicit threading guards.

## Quick Links

| Resource | Link |
|----------|------|
| Documentation | [docs.rs/chaotic_semantic_memory](https://docs.rs/chaotic_semantic_memory) |
| Crates.io | [crates.io/crates/chaotic_semantic_memory](https://crates.io/crates/chaotic_semantic_memory) |
| Issues | [GitHub Issues](https://github.com/d-o-hub/chaotic_semantic_memory/issues) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

## Important: HDC, Not Semantic Embeddings

This crate uses **Hyperdimensional Computing (HDC)** for text encoding — it is
**not** a transformer or embedding model. Understanding this distinction is critical:

| | HDC (this crate) | Transformer Embeddings (e.g. sentence-transformers) |
|---|---|---|
| **Method** | Hash-based token → random hypervector | Learned neural network encodings |
| **Similarity** | Tokens + position match → similar vectors | Semantic meaning → similar vectors |
| **"cat" vs "kitten"** | Low similarity (different tokens) | High similarity (synonyms) |
| **"cat sat" vs "sat cat"** | Different (position-aware) | Often similar |
| **Compute** | CPU-only, deterministic, no GPU | GPU-accelerated, learned weights |
| **Use case** | Keyword/lexical search, exact-match recall | Semantic search, paraphrase detection |

**Bottom line:** `inject_text` / `probe_text` match on shared tokens at similar
positions. For true semantic similarity, use an external embedding model and inject
vectors directly via `inject_concept`.

## Features

- **Hyperdimensional Computing**: 10240-bit binary hypervectors with SIMD-accelerated operations
- **Chaotic Reservoirs**: Configurable echo-state networks with spectral radius controls `[0.9, 1.1]`
- **Semantic Memory**: Concept graphs with weighted associations and similarity search
- **Optimized Retrieval**: Two-stage retrieval pipeline with heuristic-based candidate generation (bucket, graph) and dense-vector scoring.
- **Persistence**: libSQL for local SQLite or remote Turso database
- **WASM Support**: Browser-compatible with memory-based import/export
- **CLI**: Full-featured command-line interface with shell completions
- **Production-Ready**: Structured logging, metrics, input validation, memory guardrails

## Optional Analytics (DuckDB)

For advanced OLAP workloads, SQL inspection, and Parquet export, see the [`csm-duckdb`](crates/csm-duckdb) companion crate.

**Two-Crate Decision Tree:**

| Want... | Use... | Features |
|---|---|---|
| Just core memory/search | `chaotic_semantic_memory` | Lean, no C++ toolchain |
| SQL queries + CLI analytics | `csm` + `analytics` feature | Integrated experience |
| Standalone analytics binary | `csm-analytics` | Specialized for data engineers |

## Installation

### Rust Library

```bash
cargo add chaotic_semantic_memory
```

For WASM targets, build with `--target wasm32-unknown-unknown`. No additional feature flag is needed;
WASM support is enabled automatically when compiling for the `wasm32` target architecture.

```toml
[dependencies]
chaotic_semantic_memory = { version = "0.3" }
```

For library-only consumers who don't need the CLI binary or its dependencies:

```toml
[dependencies]
chaotic_semantic_memory = { version = "0.4", default-features = false }
```

### WASM npm Package (for JS/TS)

```bash
npm install @d-o-hub/chaotic_semantic_memory
```

### CLI Binary

**via npx (recommended — no install, no permission issues):**
```bash
npx @d-o-hub/csm --version
npx @d-o-hub/csm inject my-concept --database csm_memory.db
```

**via npm global install:**
```bash
npm install -g @d-o-hub/csm
```

> **Permission errors (EACCES)?** Do not use `sudo`. Configure a user-writable prefix:
> ```bash
> npm config set prefix ~/.npm-global
> export PATH=~/.npm-global/bin:$PATH  # Add to ~/.bashrc or ~/.zshrc to persist
> ```
> Or simply use `npx` above, which requires no global install and avoids permission issues.

**via cargo:**
```bash
cargo install chaotic_semantic_memory --bin csm
```

## Core Components

- `hyperdim`: binary hypervector math (`HVec10240`) and similarity operations
- `reservoir`: sparse chaotic Echo State Network mapping sequences to fixed-size hypervectors
- `singularity`: graph-based concept storage, zero-allocation caching, and semantic search
- `framework`: high-level async orchestration API, batching, and state management
- `persistence`: durable storage via libSQL and connection pooling (native only)
- `wasm`: JS-facing bindings for browser/runtime integration (wasm32 target only)
- `encoder`: text and binary encoding utilities
- `graph_traversal`: graph walk and reachability utilities
- `metadata_filter`: metadata query and filtering
- `bundle`: snapshot and bundle helpers
- `cli`: `csm` command-line utility for database management and querying
- `semantic_bridge`: Semantic Bridge Layer for concept expansion (ADR-0061)
- `bridge_retrieval`: Bridge retrieval pipeline for zero-drift semantic search
- `retrieval`: Hybrid BM25/HDC retrieval module (ADR-0062)

## Semantic Bridge Layer (v0.3.0)

Zero-drift semantic expansion on top of deterministic HDC memory:
- **CanonicalConcept**: Versioned concept with labels and related IDs
- **ConceptGraph**: In-memory label index for token-to-concept matching
- **BridgeRetrieval**: Pipeline: normalize → HDC recall → concept expansion → second recall
- **MemoryPacket**: Compressed output for LLM context injection

**Key principle**: Additive, non-destructive, never mutates canonical memory vectors.

## Hybrid BM25+HDC Retrieval (v0.3.0)

Hybrid retrieval combines keyword (BM25) and semantic (HDC) search:
- **BM25 parameters**: k1=1.2 (TF saturation), b=0.75 (doc length normalization)
- **Query-length-dependent weights**:

| Tokens | Keyword | Semantic |
|--------|---------|----------|
| 1-2    | 0.9     | 0.1      |
| 3-4    | 0.7     | 0.3      |
| 5-8    | 0.4     | 0.6      |
| 9+     | 0.2     | 0.8      |

## How Text Encoding Works (HDC Pipeline)

The built-in `TextEncoder` uses **Hyperdimensional Computing (HDC)** — a deterministic,
hash-based encoding, **not** a learned neural network:

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────────┐     ┌─────────┐
│  "hello     │     │ FNV-1a hash      │     │ Positional       │     │ Bundle  │
│   world"    │──▶ │ per token        │────▶│ permutation      │───▶│ majority│
│             │     │ PRNG → HVec10240 │     │ (word order)     │     │ rule    │
└─────────────┘     └──────────────────┘     └──────────────────┘     └─────────┘
    Tokenize             Token→HVec              Position Encode         Final HV
```

**Pipeline steps:**

1. **Tokenize**: Split on whitespace, lowercase (`hello world` → `["hello", "world"]`)
2. **Token → HVec**: FNV-1a hash → seed PRNG → generate random `HVec10240` per token
3. **Positional encoding**: Permute each token vector by its position (order matters)
4. **Bundle**: Majority-rule combination into a single `HVec10240`

**Key properties:**
- **Deterministic**: Same text always produces the same vector (FNV-1a is stable across Rust versions)
- **Token-sensitive**: Similar tokens in similar positions → similar vectors
- **NOT semantic**: Synonyms/paraphrases ("cat" vs "kitten") will NOT match
- **Position-aware**: "cat sat" ≠ "sat cat" (order matters)

### Recommended API

```rust
// HDC text encoding — good for lexical/keyword similarity
framework.inject_text("doc-1", "reservoir computing overview").await?;
let hits = framework.probe_text("reservoir computing", 5).await?;

// External embeddings — good for semantic similarity
let embedding: HVec10240 = my_model.encode("an overview of echo-state networks");
framework.inject_concept("doc-2", embedding).await?;
```

**Use `inject_text`/`probe_text` for:**
- Keyword search and exact-match recall
- Document deduplication (same/similar text)
- Indexing text where token overlap matters

**Use external embeddings (`inject_concept`) for:**
- Semantic search (synonyms, paraphrases)
- Concept-level similarity across different wording
- Cross-lingual matching

### Turso Vector Alternative

This crate uses libSQL (local SQLite or remote Turso) for persistence. For
semantic similarity, you can add Turso's [native vector search](https://turso.tech/vector)
tables alongside the crate's HDC storage using the same database:

```rust
use libsql::Builder;

// Connect to the same database this crate uses for persistence
let db = Builder::new_local("csm_memory.db").build().await?;
let conn = db.connect()?;

// Add semantic vector table alongside the crate's concepts table
conn.execute_batch("
    CREATE TABLE IF NOT EXISTS semantic_vectors (
        id TEXT PRIMARY KEY,
        embedding F32_BLOB(384)
    );
    CREATE INDEX IF NOT EXISTS semantic_idx ON semantic_vectors(
        libsql_vector_idx(embedding, 'metric=cosine')
    );
").await?;

// Query with vector_top_k
let rows = conn.query(
    "SELECT id FROM vector_top_k('semantic_idx', vector(?), 10)",
    libsql::params![query_embedding_f32_as_string]
).await?;
```

This keeps HDC and semantic vectors in the **same database**: the crate manages
`csm_concepts` and `csm_associations` tables (with `csm_` prefix for namespace isolation),
while you manage `semantic_vectors` for float-vector similarity search. Both query the same libSQL/Turso instance.

## CLI Usage

The `csm` binary provides command-line access:

```bash
# Inject a concept
csm inject my-concept --database csm_memory.db

# Find similar concepts
csm probe my-concept -k 10 --database csm_memory.db

# Create associations
csm associate source-concept target-concept --strength 0.8 --database csm_memory.db

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

    let concept = ConceptBuilder::new("cat".to_string()).build()?;
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
| `concept_cache_size` | `128` | `>= 1` | Similarity query cache capacity (set via `with_concept_cache_size`, stored separately from `FrameworkConfig`) |

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
    .with_local_db("csm_memory.db")
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

## Concurrency Model

Internal state is protected by `tokio::sync::RwLock` — safe for concurrent
access from multiple Tokio tasks via `Arc<ChaoticSemanticFramework>`.

### Multi-Instance Safety

Multiple `ChaoticSemanticFramework` instances sharing the same database file
are safe for concurrent operation:

- **Reads** (`probe`, `get_concept`, `get_associations`, `stats`) acquire
  `RwLock` read guards and can run fully concurrently across tasks and
  framework instances.
- **Writes** (`inject_concept`, `associate`, `delete_concept`) acquire write
  guards in-process and are serialized at the database layer by SQLite's WAL
  write lock. Two instances writing to the same database will queue on WAL
  without data corruption.

### SQLite WAL Mode

Local SQLite connections explicitly enable `PRAGMA journal_mode=WAL` during
initialization (`src/persistence.rs`). This provides:

- Concurrent readers never block each other or a writer.
- A single writer never blocks readers (readers see the last consistent snapshot).
- Checkpoints via `PRAGMA wal_checkpoint(TRUNCATE)` merge WAL data back into
  the main database file.

Remote Turso connections delegate concurrency to the server and do not set
WAL mode locally.

### Lock Discipline

Write locks on `singularity` are held only for in-memory operations and are
never held across `.await` points (see [ADR-0040](plans/adr/0040-async-lock-safety.md)).
Persistence I/O happens after the write lock is released, so concurrent probes
are never blocked by database writes.

### Scaling Characteristics

| Operation | Complexity | Notes |
|---|---|---|
| `inject_concept` | O(1) amortized | HashMap insert + dense vector append |
| `associate` | O(1) amortized | HashMap insert with optional eviction |
 `probe` (exact scan) | O(n) | Cosine similarity over all n concepts; parallelized via Rayon on native. Zero-copy caching: uses hashed keys and `Arc` to share cache results. |
| `probe` (bucket candidates) | O(n / 2^w) | w-bit bucket width narrows candidate set before exact scoring |
| `probe` (graph candidates) | O(f^d) | BFS from nearest neighbor at depth d, fanout f |

The default retrieval path is an **exact O(n) scan** over all stored concept
vectors. For larger corpora, two-stage candidate generation can be enabled via
`RetrievalConfig`:

- **Bucket candidates**: Coarse hash-bucketing on the first w bits of the
  hypervector narrows the candidate set before exact scoring.
- **Graph candidates**: BFS expansion from the nearest-neighbor seed through
  the association graph, bounded by depth and fanout.

Both reduce the scored subset from n to a smaller candidate set while
preserving exact similarity semantics on the reranking pass.

### ANN/LSH Deferred

Approximate nearest-neighbor (ANN) or locality-sensitive hashing (LSH) indexing
is **intentionally deferred** until benchmarks demonstrate latency regression
beyond the current threshold. As documented in
[ADR-0056](plans/adr/0056-performance-follow-up-priorities.md), the trigger is
**>200k concepts** with latency degradation. Current benchmarks show the exact
scan completes in ~24ms at 200k concepts, well within acceptable bounds
(see [ADR-0059](plans/adr/0059-retrieval-optimization.md) for retrieval
optimization details and benchmark methodology).

### Async Runtime

The framework is fully async. Do **not** wrap calls in `block_on` inside an
existing Tokio runtime — use `.await` directly or spawn a task. All public
APIs return `Result<T, MemoryError>` and use Tokio for I/O, with Rayon
gated behind `#[cfg(not(target_arch = "wasm32"))]` for CPU parallelism.

## Development Gates

```bash
cargo check --quiet
cargo test --all-features --quiet
cargo fmt --check --quiet
cargo clippy --quiet -- -D warnings
```

LOC policy: each source file in `src/` and `crates/` must stay at or below 500 lines.

### Additional Testing

**Mutation Testing**:
```bash
cargo install cargo-mutants
scripts/mutation_test.sh fast
```

**Fuzz Testing**:
```bash
cargo +nightly fuzz run hvec_from_bytes
```

### Real Usage Validation

```bash
# CLI workflow test (inject → probe → export → import)
csm inject doc-1 --database test.db
csm probe doc-1 -k 5 --database test.db
csm export -o backup.json --database test.db
csm import backup.json --merge --database test.db

# Skill-memory integration test
# Location: .agents/csm-memory/skill-memory.db
# Verified: concept save/recall, db file creation
```

### Benchmark & Performance Gates

```bash
cargo bench --bench benchmark -- --save-baseline main
cargo bench --bench benchmark -- --baseline main
cargo bench --bench persistence_benchmark -- --save-baseline main
cargo bench --bench persistence_benchmark -- --baseline main
```

Primary perf gate: `reservoir_step_50k < 100us`.

## License

[MIT](LICENSE)

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for:
- Code style and linting requirements
- Test and benchmark commands
- Pull request process
- ADR submission for architectural changes
