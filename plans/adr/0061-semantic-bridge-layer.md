# ADR-0061: Semantic Bridge Layer as Overlay on Singularity

## Status
Proposed

## Context

GitHub Issue #52 proposes a "Zero-Drift Semantic Bridge Layer" to add semantic
generalization on top of the existing deterministic HDC memory system. The goal
is to improve recall for semantically equivalent queries (e.g., "agent memory"
≈ "cross-session context") without introducing embedding drift, reindexing, or
vendor lock-in.

### Existing Capabilities (reusable)

| Capability | Module | Status |
|---|---|---|
| Vector-bearing concept storage | `singularity.rs` | Complete |
| Weighted association graph | `singularity.rs` | Complete |
| Graph traversal (BFS, Dijkstra) | `graph_traversal.rs` | Complete |
| Candidate retrieval (graph/bucket/exact) | `singularity_retrieval.rs` | Complete |
| Deterministic text encoding (FNV-1a) | `encoder.rs` | Complete |
| Metadata-filtered search | `metadata_filter.rs` | Complete |
| Persistence + versioning | `persistence.rs`, `persistence_versions.rs` | Complete |

### New Capabilities Required

1. **Canonical concept graph** with aliases, labels, and token→concept index
2. **Memory grounding** (linking stored memories to canonical concepts)
3. **Bridge retrieval pipeline** (normalize → recall → expand → second recall)
4. **Optional semantic reranker** (pluggable trait, no default LLM dependency)
5. **Multi-score breakdown** (deterministic, concept, semantic, final)
6. **Memory packet compiler** (compressed output for LLM context injection)

## Decision

### 1. Layer on top of Singularity — do not replace it

The semantic bridge is an **additive overlay**. Canonical memory (Singularity)
remains the source of truth for stored vectors and associations. The bridge
layer adds symbolic canonicalization and retrieval orchestration without
mutating stored data.

### 2. Use `CanonicalConcept` — do not introduce a second `Concept` type

The existing `singularity::Concept` is a public, re-exported type. Adding
another public `Concept` would create API confusion. The canonical graph uses
`CanonicalConcept` with fields: `id`, `version`, `labels`, `related`.

### 3. Keep canonical graph separate from `associations`

The existing `Singularity.associations` stores weighted similarity/association
edges. The canonical concept graph stores **symbolic identity relationships**
(aliases, relatedness). Mixing them would conflate two different edge semantics.

### 4. Reuse encoder normalization as single tokenization source

Bridge tokenization must use `TextEncoder` (or extracted helpers from
`encoder.rs`) to ensure token consistency between encoding and bridge recall.
No separate normalization code.

### 5. Add memory grounding field to `Concept`

Add `canonical_concept_ids: Vec<String>` (with `#[serde(default)]`) to the
stored `Concept` struct. This links memories to canonical concepts without
breaking backward compatibility.

### 6. Keep semantic reranking optional

Define a `SemanticReranker` trait with `rerank()` and `version()` methods. The
default pipeline runs without a reranker. Implementations can wrap local models,
remote APIs, or rule-based heuristics.

### 7. Use `singularity_retrieval` as reusable scorer, not the bridge pipeline

The existing exact scan and scored candidate retrieval are reused for the
vector-similarity scoring stage. The bridge pipeline orchestrates the
higher-level multi-stage flow.

## Module Decomposition

All new modules respect the 500 LOC hard limit.

### New Modules

| Module | Purpose | Est. LOC |
|---|---|---|
| `src/semantic_bridge.rs` | Public types: `CanonicalConcept`, `ScoreBreakdown`, `MemoryPacket`, `BridgeConfig`, `SemanticReranker` trait | 180–280 |
| `src/concept_graph.rs` | In-memory canonical graph: alias index, token→concept index, labeled relations, expansion | 250–350 |
| `src/bridge_retrieval.rs` | Pipeline: normalize → deterministic recall → concept expansion → second recall → rerank → score assembly | 250–400 |
| `src/bridge_persistence.rs` | Feature-gated persistence: bridge schema, save/load canonical graph, grounding field serialization | 180–300 |

### Existing Module Changes (minimal)

| Module | Change | LOC Impact |
|---|---|---|
| `singularity.rs` | Add `canonical_concept_ids: Vec<String>` field with `#[serde(default)]` | +3 |
| `concept_builder.rs` | Add `with_canonical_concepts()` builder method | +10 |
| `encoder.rs` | Extract `pub fn tokenize()` helper for bridge reuse | +5 |
| `export_payload.rs` | Include new field in export/import conversion | +5 |
| `lib.rs` | Register and export new modules | +10 |

## Implementation Phases

### Phase 55: Core Types & Canonical Graph (cost: 10)
- ADR-0061 accepted
- `semantic_bridge.rs`: public types
- `concept_graph.rs`: in-memory graph with YAML/JSON loader
- Token→concept index
- Basic concept expansion

### Phase 56: Bridge Retrieval Pipeline (cost: 12)
- `bridge_retrieval.rs`: full pipeline
- Memory grounding field on `Concept`
- `SemanticReranker` trait definition
- Multi-score breakdown
- Integration with `singularity_retrieval` for vector scoring

### Phase 57: Memory Packet & Compression (cost: 6)
- Packet compiler in `bridge_retrieval.rs` or `semantic_bridge.rs`
- Token-budget-aware compression
- Structured output format for LLM injection

### Phase 58: Bridge Persistence (cost: 8)
- `bridge_persistence.rs`: schema migration, save/load
- Export/import integration
- Backfill/rebuild index utilities

## Consequences

### Positive
- Zero embedding drift — canonical memory is deterministic
- No vendor lock-in — semantic reranking is optional and pluggable
- No mutation of stored vectors — bridge is additive
- Backward compatible — `#[serde(default)]` on new fields
- Controlled complexity — 4 new modules, <500 LOC each

### Negative
- Additional schema migration for bridge tables
- Concept graph must be maintained alongside memory
- Two retrieval paths (direct probe vs bridge) may confuse users initially

### Risks
- Normalization drift if bridge bypasses `TextEncoder`
- Over-engineering if full pipeline built before validating recall improvement

## Alternatives Considered

1. **Embed aliases in metadata**: Simpler but no graph traversal or expansion
2. **Replace Singularity with new engine**: Too disruptive, loses proven code
3. **Use external embedding model**: Introduces drift, vendor lock-in, compute cost
4. **Extend existing associations**: Conflates symbolic and similarity semantics
