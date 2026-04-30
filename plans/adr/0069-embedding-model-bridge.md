# ADR-0069: External Embedding Model Bridge

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`TextEncoder` (src/encoder.rs) generates hypervectors via FNV-1a hashing + seeded PRNG. This is:
- Deterministic ✅
- Fast (~µs per text) ✅
- WASM-compatible ✅
- **Semantically blind** ❌ — "king" and "monarch" hash to orthogonal HVs

Modern memory systems use learned embeddings (sentence-transformers, OpenAI ada-002, Voyage, Cohere) so semantically similar text retrieves correctly. The Semantic Bridge Layer (ADR-0061) compensates with BM25+HDC blending but cannot bridge true synonymy.

We need a bridge that lets users plug in an embedding model when semantic accuracy matters.

## Decision Drivers

- Optional — pure-HDC users keep zero-dependency core
- WASM build must not break (embedding model is native-only)
- Must accept any dimensionality (project to HVec10240 or store native)
- Local + remote backends supported
- LOC budget ≤ 500/file

## Considered Options

1. **Single trait + 3 backends** (`fastembed-rs`, `candle`, `ort`) bundled behind features
2. Trait only — users implement their own backend
3. HTTP client only (OpenAI / Voyage) — no local
4. Replace TextEncoder entirely

## Decision Outcome

Chosen: **Option 1** — trait + bundled backends. Provides ergonomic out-of-box experience while preserving extensibility.

## Implementation

### New module

```
src/embedding/
  mod.rs              # EmbeddingProvider trait + projection
  hdc_text.rs         # current TextEncoder, refactored as backend
  fastembed.rs        # local ONNX models via fastembed-rs
  remote_openai.rs    # HTTP client for OpenAI embeddings
  remote_voyage.rs    # HTTP client for Voyage embeddings
  projection.rs       # f32[n] → HVec10240 random projection (Achlioptas/sparse)
```

Each file ≤ 400 LOC.

### Trait

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn native_dim(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}
```

### Projection layer

Random sparse projection (Achlioptas) maps native_dim → 10240 with theoretical guarantees on cosine preservation (Johnson-Lindenstrauss).

```rust
pub struct Projection {
    matrix: SparseMatrix,  // 10240 × native_dim, values ∈ {-1, 0, +1}
    seed: u64,             // for reproducibility
}

impl Projection {
    pub fn project(&self, vec: &[f32]) -> HVec10240;
}
```

### Framework integration

```rust
pub struct Framework { ... }

impl FrameworkBuilder {
    pub fn with_embedding_provider<P: EmbeddingProvider + 'static>(self, p: P) -> Self;
}

impl Framework {
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()>;
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>>;
    // Both already exist; rewire to use embedding provider when configured
}
```

If no provider configured → falls back to existing FNV-1a TextEncoder (zero behavior change).

### Cargo features

```toml
[features]
embed-fastembed = ["dep:fastembed", "dep:tokio"]
embed-openai = ["dep:reqwest"]
embed-voyage = ["dep:reqwest"]
```

WASM target gates these out at compile time.

### CLI

```
csm inject "concept-id" --text "long text" --use-embeddings
csm probe-text "query" -k 5 --provider fastembed:bge-small
```

## Pros and Cons

### Pros
- Fixes "king/monarch" semantic blindness when needed
- Backward compatible
- Composable: HDC algebra (bind/bundle) still works on projected vectors
- WASM stays untouched

### Cons
- Adds optional but heavy deps (fastembed ~200 MB models)
- Projection introduces small accuracy loss vs storing native dim
- Async embedding adds latency vs FNV-1a

## Acceptance Criteria

- [ ] All 4 backends implement `EmbeddingProvider` trait
- [ ] Default behavior unchanged (no feature → FNV-1a TextEncoder)
- [ ] Projection roundtrip preserves cosine similarity ≥ 0.9 for known pairs
- [ ] CLI flag works for at least fastembed backend
- [ ] WASM build succeeds with no embedding features
- [ ] All `src/embedding/*.rs` files ≤ 400 LOC
