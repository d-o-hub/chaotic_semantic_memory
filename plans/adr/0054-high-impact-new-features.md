# [ADR-0054] High-Impact New Features: Text Encoding, Filtered Search, Graph Traversal

## Status
Proposed

## Context and Problem Statement

Analysis of the crate reveals three high-impact feature gaps that prevent real-world adoption:

1. **No built-in text encoding**: Every example uses `HVec10240::random()` as a placeholder.
   Users cannot meaningfully use the crate for AI memory without bringing their own
   embedding pipeline. This makes the crate unusable for WASM/edge and hard to evaluate.

2. **No metadata-filtered similarity search**: Users must `probe()` then fetch each concept
   individually to filter by metadata. This is O(N) extra work for the most common RAG pattern.

3. **No association graph traversal**: Associations are stored but only outbound edges are
   queryable. No BFS, shortest path, or multi-hop retrieval — limiting knowledge graph use cases.

4. **No incremental bundling**: `bundle()` is a one-shot operation. Streaming/sliding-window
   memory requires building and updating bundle state over time.

5. **No change event hooks**: External systems cannot react to memory mutations without polling.

## Decision Drivers
- **User Impact**: Text encoding unlocks every example from toy to functional
- **Competitive Parity**: Vector DBs offer filtered search; knowledge graphs offer traversal
- **Zero New Dependencies**: HDC text encoding requires no ML framework
- **WASM Compatibility**: All features must work in WASM target
- **AGENTS.md**: 500 LOC limit per file, all public APIs return Result<T, Error>

## Considered Options
- **Option A**: Ship encoder as separate crate → Fragmented UX, harder discovery
- **Option B**: Built-in encoder with feature flag → Best DX, single dependency
- **Option C**: Only document how to bring external embeddings → No improvement

## Decision Outcome
Chosen option: **Option B** — built-in text encoder + filtered search + graph traversal,
implemented as Phases 37-39 of Wave 15.

### Phase 37: Text-to-Hypervector Encoding (cost: 8)

**Core**: Deterministic text → HVec10240 encoder using HDC principles (no ML deps).

Algorithm:
1. **Tokenize**: whitespace + lowercase + optional unicode segmentation
2. **Token → base HVec**: stable hash (FNV-1a) → seeded PRNG → random HVec10240
3. **Position encoding**: `token_hv.permute(position * stride)`
4. **Bundle**: majority-rule bundling of all position-encoded token vectors
5. **Optional**: character n-gram overlay for typo robustness

API:
```rust
pub struct TextEncoder { config: TextEncoderConfig }
impl TextEncoder {
    pub fn new() -> Self;
    pub fn with_config(config: TextEncoderConfig) -> Self;
    pub fn encode(&self, text: &str) -> HVec10240;
    pub fn encode_with_ngrams(&self, text: &str, n: usize) -> HVec10240;
}

// Framework convenience
impl ChaoticSemanticFramework {
    pub async fn inject_text(&self, id: &str, text: &str) -> Result<()>;
    pub async fn inject_text_with_metadata(&self, id: &str, text: &str, metadata: HashMap<...>) -> Result<()>;
    pub async fn probe_text(&self, query: &str, top_k: usize) -> Result<Vec<(String, f32)>>;
}
```

New files: `src/encoder.rs` (~200 LOC)

**Impact**: HIGH — Every example becomes functional. WASM users get zero-dep encoding.
Self-contained crate for AI memory without external embedding pipeline.

### Phase 38: Metadata-Filtered Similarity Search (cost: 6)

**Core**: Filter concepts during similarity search using metadata predicates.

API:
```rust
pub enum MetadataFilter {
    Eq(String, serde_json::Value),           // key == value
    In(String, Vec<serde_json::Value>),      // key in [values]
    Exists(String),                           // key exists
    And(Vec<MetadataFilter>),                 // all must match
    Or(Vec<MetadataFilter>),                  // any must match
    Not(Box<MetadataFilter>),                 // negation
}

impl Singularity {
    pub fn find_similar_filtered(&self, query: &HVec10240, top_k: usize, filter: &MetadataFilter) -> Vec<(String, f32)>;
}

impl ChaoticSemanticFramework {
    pub async fn probe_filtered(&self, query: HVec10240, top_k: usize, filter: MetadataFilter) -> Result<Vec<(String, f32)>>;
}
```

Implementation: predicate-based scan during similarity computation (skip non-matching concepts
before computing cosine similarity). No new data structures needed for initial version.

New files: `src/metadata_filter.rs` (~150 LOC)

**Impact**: HIGH — RAG with document scoping, per-session chat memory, multi-tenant filtering.
Most requested pattern in vector database usage.

### Phase 39: Association Graph Traversal (cost: 8)

**Core**: Multi-hop graph queries on the association graph.

API:
```rust
pub struct TraversalConfig {
    pub max_depth: usize,              // Maximum hops (default: 3)
    pub min_strength: f32,             // Minimum edge strength (default: 0.0)
    pub max_results: usize,            // Maximum nodes to visit (default: 100)
}

impl Singularity {
    pub fn neighbors(&self, id: &str, min_strength: f32) -> Vec<(String, f32)>;
    pub fn bfs(&self, start: &str, config: &TraversalConfig) -> Vec<(String, u32)>;  // (id, depth)
    pub fn shortest_path(&self, from: &str, to: &str, config: &TraversalConfig) -> Option<Vec<String>>;
    pub fn incoming_associations(&self, id: &str) -> Vec<(String, f32)>;
}

impl ChaoticSemanticFramework {
    pub async fn traverse(&self, start: &str, config: TraversalConfig) -> Result<Vec<(String, u32)>>;
    pub async fn shortest_path(&self, from: &str, to: &str) -> Result<Option<Vec<String>>>;
}
```

Implementation:
- BFS/DFS with visited set and depth tracking
- Shortest path via Dijkstra with cost = `-ln(strength)` (guard against 0)
- `incoming_associations` via reverse adjacency map (built lazily on first call)

New files: `src/graph_traversal.rs` (~200 LOC)

**Impact**: HIGH — Enables knowledge graph navigation, reasoning chains, agent tool memory,
topic exploration. Turns associations from a storage primitive into a queryable graph.

### Phase 40: Incremental Bundle Accumulator (cost: 4)

**Core**: Streaming bundle state that supports add/remove/finalize.

API:
```rust
pub struct BundleAccumulator {
    counts: Box<[i32; HVec10240::DIMENSION]>,
    n: u32,
}

impl BundleAccumulator {
    pub fn new() -> Self;
    pub fn add(&mut self, hv: &HVec10240);
    pub fn remove(&mut self, hv: &HVec10240);
    pub fn finalize(&self) -> HVec10240;
    pub fn len(&self) -> u32;
}
```

Implementation: Maintain signed counters per bit. `add` increments, `remove` decrements,
`finalize` applies majority threshold. Same approach as `bundle()` but reusable.

New code in: `src/hyperdim.rs` (~60 LOC addition, file stays under 500 LOC)

**Impact**: MEDIUM — Enables sliding-window memory, dynamic "working memory" summaries,
incremental document indexing, real-time concept drift tracking.

### Phase 41: Memory Change Events (cost: 4)

**Core**: Observable memory mutations via broadcast channel.

API:
```rust
pub enum MemoryEvent {
    ConceptInjected { id: String, timestamp: u64 },
    ConceptUpdated { id: String, timestamp: u64 },
    ConceptDeleted { id: String, timestamp: u64 },
    Associated { from: String, to: String, strength: f32 },
    Disassociated { from: String, to: String },
}

impl ChaoticSemanticFramework {
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<MemoryEvent>;
}
```

New code in: `src/framework.rs` (~40 LOC addition)

**Impact**: MEDIUM — Enables reactive UIs, external index updates, memory consolidation
pipelines, audit logging, replication.

### Positive Consequences
- Crate becomes a self-contained AI memory system (no external embedding required)
- All 7 examples become functional with real semantic similarity
- WASM users get full text→memory pipeline in the browser
- Knowledge graph use case becomes viable with traversal APIs
- Competitive with qdrant/weaviate for lightweight deployments

### Negative Consequences
- Text encoder quality won't match transformer embeddings (documented as lightweight/local)
- Metadata filtering adds ~5% overhead to similarity scan (predicate check per concept)
- Graph traversal on large association graphs may need pagination
- ~600 LOC across 3 new files

## Implementation Priority
P1 (Highest Impact): Phase 37 — Text Encoder (unblocks every example)
P2 (High Impact): Phase 38 — Metadata Filtered Search (most common real pattern)
P3 (High Impact): Phase 39 — Graph Traversal (knowledge graph use case)
P4 (Medium Impact): Phase 40 — Bundle Accumulator (streaming memory)
P5 (Medium Impact): Phase 41 — Memory Events (reactive patterns)

## Validation Criteria
- Text encoder produces deterministic output (golden tests with fixed input)
- Text encoder similarity: similar texts > 0.6, dissimilar texts < 0.3
- Filtered search matches brute-force filter-then-sort
- BFS visits correct nodes at correct depths (tested on known graph)
- Bundle accumulator add/remove/finalize matches `bundle()` output
- All files under 500 LOC
- WASM target compiles with all new features
