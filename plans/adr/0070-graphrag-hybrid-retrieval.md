# ADR-0070: GraphRAG Hybrid Retrieval

## Status

Proposed (2026-04-30)

## Context and Problem Statement

We have:
- `probe()` — vector similarity (semantic neighbors)
- `traverse()` / `shortest_path()` — graph BFS over associations
- `bridge_retrieval` — semantic bridge with BM25+HDC blend

We do **not** have:
- A unified retriever that returns "concepts similar to query AND their N-hop neighbors"
- Score blending across similarity and graph distance
- "Anchor" retrieval (find best N anchors → traverse from each → rerank by joint score)

This pattern, popularized by Microsoft GraphRAG and LlamaIndex KnowledgeGraphIndex, is the dominant mode for memory systems where associations carry meaning (chat threads, project files, knowledge bases).

## Decision Drivers

- Reuse existing `probe` + `traverse` primitives (no duplication)
- Keep retrieval pipeline composable
- Single new public API: `probe_with_graph()`
- Configurable scoring weights
- LOC budget ≤ 400/file

## Considered Options

1. **Pipeline retriever** — anchor probe → traverse → joint rank
2. Random-walk-based reranker (Personalized PageRank)
3. Just document the pattern without code

## Decision Outcome

Chosen: **Option 1** — explicit pipeline retriever. PPR is overkill for typical N≤10 hop graphs and harder to debug.

## Implementation

### New module

`src/retrieval/graph_rag.rs` (≤ 400 LOC)

### Public API

```rust
pub struct GraphRagConfig {
    pub anchor_top_k: usize,        // probe top-K to seed
    pub max_hops: usize,            // BFS depth from each anchor
    pub min_assoc_strength: f32,    // edge threshold
    pub similarity_weight: f32,     // 0.0..1.0
    pub graph_weight: f32,          // 0.0..1.0
    pub final_top_k: usize,         // top-K after rerank
}

impl Framework {
    pub async fn probe_with_graph(
        &self,
        query: HVec10240,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>>;

    pub async fn probe_text_with_graph(
        &self,
        text: &str,
        config: GraphRagConfig,
    ) -> Result<Vec<GraphRagResult>>;
}

pub struct GraphRagResult {
    pub id: String,
    pub score: f32,                 // joint score
    pub similarity: f32,            // raw cosine to query
    pub anchor_id: Option<String>,  // which anchor reached this
    pub hop_distance: usize,        // 0 = anchor itself
    pub assoc_strength: f32,        // path strength
}
```

### Algorithm

1. **Anchor**: `probe(query, anchor_top_k)` → seed set
2. **Expand**: for each anchor, `traverse(anchor, max_hops, min_assoc_strength)`
3. **Score**: for each unique result `r`:
   ```
   score(r) = similarity_weight * cosine(query, r.vector)
            + graph_weight * (1.0 / (1 + r.hop_distance)) * r.path_strength
   ```
4. **Dedupe + rank** by score, return `final_top_k`

### CLI

```
csm probe-graph "query text" --anchors 5 --hops 2 --top-k 20 --weights 0.6,0.4
```

### Tests

- `tests/graph_rag.rs`:
  - Synthetic graph with known structure
  - Verify anchor concept is rank 0 (hop 0)
  - Verify connected siblings outrank disconnected high-cosine matches
  - Edge cases: empty graph, isolated anchors, cycles

## Pros and Cons

### Pros
- Reuses existing primitives (no new index, no new persistence)
- Drastically improves retrieval quality for graph-heavy memory
- Mirrors GraphRAG / KG-RAG industry patterns

### Cons
- Quadratic-ish cost at large `anchor_top_k * max_hops`
- Weight tuning is workload-dependent
- May surface "weakly related" hits that pure similarity would have filtered

## Acceptance Criteria

- [ ] `probe_with_graph` and `probe_text_with_graph` implemented
- [ ] CLI subcommand `probe-graph`
- [ ] WASM bindings exposed
- [ ] Synthetic-graph tests pass
- [ ] Bench `probe_graph_1k_concepts_5_hops` ≤ 20 ms p50
- [ ] `src/retrieval/graph_rag.rs` ≤ 400 LOC
