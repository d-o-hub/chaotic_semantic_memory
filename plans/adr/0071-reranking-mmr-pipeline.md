# ADR-0071: Reranking + MMR Pipeline

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`probe()` returns top-K by raw cosine. This is fine for nearest-neighbor lookup but fails common retrieval needs:

- **Diversity** — top-5 may all be near-duplicates of the same concept cluster.
- **Recency** — older but still-similar concepts dominate fresh memory.
- **Precision** — coarse cosine ranking + cheap re-rank often beats one big embedding.

Industry standard is to retrieve K' > K with a fast scorer, then rerank K' → K with a quality-aware function (MMR for diversity, cross-encoder for accuracy, time-decay for memory systems).

## Decision Drivers

- Composable: each reranker is an opt-in stage
- No new heavy dependencies (cross-encoder is optional under feature flag)
- Stable scores in `[0, 1]` range
- LOC budget ≤ 350/file

## Considered Options

1. **Reranker trait + 3 implementations**: MMR, RecencyDecay, CrossEncoder
2. Hard-code MMR into `probe`
3. Separate crate

## Decision Outcome

Chosen: **Option 1** — trait + 3 implementations. Allows composition (MMR → recency).

## Implementation

### New module

`src/retrieval/rerank.rs` (≤ 350 LOC)

### Trait

```rust
pub trait Reranker: Send + Sync {
    fn name(&self) -> &str;
    fn rerank(
        &self,
        query: &HVec10240,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Vec<RerankCandidate>;
}

pub struct RerankCandidate {
    pub id: String,
    pub vector: Arc<HVec10240>,
    pub metadata: ConceptMetadata,
    pub score: f32,             // updated by each reranker stage
    pub created_at_unix: i64,
}
```

### Built-in rerankers

#### MMR (Maximum Marginal Relevance)

```rust
pub struct MmrReranker { pub lambda: f32 } // 0.0 = full diversity, 1.0 = no diversity
```

Standard MMR formula:
```
score(c) = λ * sim(c, query) - (1-λ) * max_{s∈selected} sim(c, s)
```

#### RecencyDecay

```rust
pub struct RecencyDecayReranker {
    pub half_life_days: f32,
    pub blend: f32,  // 0.0 = pure recency, 1.0 = pure similarity
}
```

Score:
```
age_days = (now - created_at) / 86400
recency = 0.5 ^ (age_days / half_life)
score' = blend * score + (1 - blend) * recency
```

#### CrossEncoder (opt-in feature)

```rust
#[cfg(feature = "rerank-cross")]
pub struct CrossEncoderReranker { model: CandleModel, ... }
```

Uses `candle` to load a small cross-encoder ONNX model (e.g., `ms-marco-MiniLM-L-6-v2`).

### Pipeline integration

```rust
impl Framework {
    pub async fn probe_with_rerankers(
        &self,
        query: HVec10240,
        initial_top_k: usize,
        rerankers: &[Box<dyn Reranker>],
        final_top_k: usize,
    ) -> Result<Vec<(String, f32)>>;
}
```

### CLI

```
csm probe "query text" -k 5 --initial-k 50 --rerank mmr:0.7,recency:30d
```

## Pros and Cons

### Pros
- Drop-in quality boost without changing index
- Composable — chain MMR → recency → cross-encoder
- Cross-encoder is opt-in (no deps for users who don't want it)

### Cons
- Each stage adds latency (negligible for MMR/recency, ~10ms for cross-encoder)
- MMR has O(K²) complexity within the rerank window

## Acceptance Criteria

- [ ] MMR + RecencyDecay shipped (no new deps)
- [ ] CrossEncoder behind `rerank-cross` feature
- [ ] `probe_with_rerankers` API
- [ ] CLI `--rerank` flag with parser
- [ ] Tests verify diversity (MMR) and time-weighting (recency)
- [ ] `src/retrieval/rerank.rs` ≤ 350 LOC
