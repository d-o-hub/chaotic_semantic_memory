# GOAP Plan: Zero-Drift Semantic Bridge Layer (Issue #52)

**Issue:** https://github.com/d-o-hub/chaotic_semantic_memory/issues/52
**ADR:** [ADR-0061](adr/0061-semantic-bridge-layer.md)
**Current State:** Core HDC memory system complete, no symbolic semantic expansion
**Target State:** Additive semantic bridge layer with concept graph, expanded retrieval, and LLM-ready output

---

## Current World State

```yaml
semantic_bridge:
  adr_0061_status: implemented
  concept_graph_exists: true
  bridge_types_defined: true
  bridge_retrieval_pipeline: true
  memory_grounding_field: true
  semantic_reranker_trait: true
  memory_packet_output: true
  bridge_persistence: true
  bridge_tests: true
  bridge_benchmarks: true

  # Existing infrastructure (reuse — do NOT rebuild)
  existing_reusable:
    hdc_recall: true                    # singularity.rs find_similar/find_similar_cached
    graph_traversal: true               # graph_traversal.rs BFS/Dijkstra/neighbors
    candidate_retrieval: true           # singularity_retrieval.rs graph/bucket/exact
    text_encoder: true                  # encoder.rs FNV-1a, position encoding, n-grams
    hybrid_scoring: true                # retrieval/hybrid.rs normalize + merge + weights
    bm25_keyword_search: true           # retrieval/bm25.rs full BM25 index
    metadata_filter: true               # metadata_filter.rs Eq/In/Exists/And/Or/Not
    persistence_versioning: true        # persistence.rs + persistence_versions.rs
    framework_probe: true               # framework.rs probe/probe_text/probe_filtered

  # LOC budget (500 max per file)
  loc_budget:
    singularity_rs: 491                 # +3 safe → 494
    concept_builder_rs: 154             # +10 safe → 164
    encoder_rs: 474                     # +5 safe → 479
    lib_rs: 201                         # +8 safe → 209
    export_payload_rs: 156              # +5 safe → 161
    framework_rs: 474                   # +0 untouched
```

---

## Target State

```yaml
semantic_bridge:
  adr_0061_status: accepted
  concept_graph_exists: true
  bridge_types_defined: true
  bridge_retrieval_pipeline: true
  memory_grounding_field: true
  semantic_reranker_trait: true
  memory_packet_output: true
  bridge_persistence: true
  bridge_tests: true
  bridge_benchmarks: true
```

---

## Architecture: Key Decisions

### 1. ConceptGraph ≠ Singularity associations

| Aspect | Singularity associations | ConceptGraph |
|---|---|---|
| Edge type | Weighted similarity (f32) | Symbolic identity / relatedness |
| Purpose | "A is similar to B at strength 0.8" | "agent memory = session memory = ai memory" |
| Expansion | Candidate generation for retrieval | Query-time synonym resolution |
| Mutation | Updated by `associate()` | Updated by explicit graph management |

**Decision:** Separate graph. Mixing would corrupt both semantics.

### 2. Bridge is additive — canonical memory is untouched

```
query
  → normalize (shared tokenizer from encoder.rs)
  → deterministic recall (existing Singularity.find_similar)
  → concept expansion (NEW: ConceptGraph.match_tokens → expand)
  → second recall (existing Singularity.find_similar with expanded vector)
  → merge + score breakdown (reuse retrieval/hybrid.rs)
  → optional rerank (NEW: SemanticReranker trait)
  → compile MemoryPacket (NEW)
  → output
```

### 3. Reranker is optional — no default LLM dependency

The `SemanticReranker` trait is sync and object-safe. No default implementation
ships. Users bring their own (local model, remote API, or rule-based).

---

## Actions

### Phase 55: Core Types & Canonical Graph (cost: 10)

```yaml
preconditions:
  adr_0061_status: proposed
effects:
  adr_0061_status: accepted
  bridge_types_defined: true
  concept_graph_exists: true
```

- name: accept_adr_0061
  - file: plans/adr/0061-semantic-bridge-layer.md
  - cost: 0
  - description: |
      Change ADR-0061 status from "Proposed" to "Accepted".
      Update ADR_REGISTRY.md with accepted status.

- name: create_semantic_bridge_types
  - file: src/semantic_bridge.rs (NEW, ~200 LOC)
  - cost: 3
  - description: |
      Define public types for the semantic bridge layer:
      - `CanonicalConcept { id: String, version: u32, labels: Vec<String>, related: Vec<String> }`
      - `BridgeConfig { max_expansion_depth: u8, max_packet_facts: usize, token_budget: usize }`
      - `ScoreBreakdown { deterministic: f32, concept: f32, semantic: f32, final_score: f32, evidence: Vec<String> }`
      - `BridgeHit { id: String, text_preview: Option<String>, scores: ScoreBreakdown }`
      - `MemoryPacket { query_intent: String, facts: Vec<String>, sources: Vec<String>, confidence: f32 }`
      - `trait SemanticReranker { fn version(&self) -> &str; fn rerank(&self, query: &str, hits: &mut [BridgeHit]); }`
      All types derive Serialize/Deserialize where applicable.

- name: create_concept_graph
  - file: src/concept_graph.rs (NEW, ~300 LOC)
  - cost: 5
  - description: |
      In-memory canonical concept graph with:
      - `ConceptGraph { concepts: HashMap<String, CanonicalConcept>, label_index: HashMap<String, Vec<String>> }`
      - `new()` — empty graph
      - `add_concept(concept: CanonicalConcept)` — index all labels
      - `remove_concept(id: &str)` — clean up label index
      - `get_concept(id: &str) -> Option<&CanonicalConcept>`
      - `match_tokens(tokens: &[String]) -> Vec<String>` — lookup token→concept_ids via label_index
      - `expand(concept_ids: &[String]) -> Vec<String>` — collect labels from matched concepts + related concepts
      - `load_from_json(reader: impl Read) -> Result<ConceptGraph>` — deserialize from JSON
      - `save_to_json(writer: impl Write) -> Result<()>` — serialize to JSON
      - `concept_count() -> usize`
      - `label_count() -> usize`
      Label index is rebuilt on add/remove. Labels are lowercased for case-insensitive matching.
      `expand()` follows `related` edges one level deep (configurable via BridgeConfig.max_expansion_depth).

- name: extract_shared_tokenizer
  - file: src/encoder.rs (+5 LOC)
  - cost: 1
  - description: |
      Make tokenization reusable by bridge:
      - Change `fn tokenize_code(text: &str) -> Vec<String>` from private to `pub fn tokenize_code(text: &str) -> Vec<String>`
      - Add `pub fn tokenize(text: &str, code_aware: bool, lowercase: bool) -> Vec<String>` convenience function
      This ensures bridge and encoder use identical normalization. No logic change.

- name: register_bridge_modules
  - file: src/lib.rs (+8 LOC)
  - cost: 1
  - description: |
      Add module declarations and re-exports:
      - `pub mod concept_graph;`
      - `pub mod semantic_bridge;`
      - Re-export key types: `ConceptGraph`, `CanonicalConcept`, `BridgeHit`, `MemoryPacket`, `ScoreBreakdown`
      - Add to prelude: `ConceptGraph`, `BridgeHit`, `MemoryPacket`

### Phase 56: Bridge Retrieval Pipeline (cost: 12)

```yaml
preconditions:
  bridge_types_defined: true
  concept_graph_exists: true
effects:
  bridge_retrieval_pipeline: true
  memory_grounding_field: true
  semantic_reranker_trait: true
```

- name: create_bridge_retrieval
  - file: src/bridge_retrieval.rs (NEW, ~350 LOC)
  - cost: 7
  - description: |
      Retrieval pipeline orchestrator:
      - `BridgeRetrieval { encoder: TextEncoder, concept_graph: ConceptGraph, config: BridgeConfig }`
      - `pub fn query(singularity: &Singularity, query_text: &str, top_k: usize, reranker: Option<&dyn SemanticReranker>) -> Result<Vec<BridgeHit>>`
      Pipeline steps:
        1. Normalize: `encoder::tokenize(query_text, config.code_aware, true)`
        2. Encode: `encoder.encode(query_text)` → primary query HVec
        3. First recall: `singularity.find_similar(&query_hv, top_k)` — deterministic scores
        4. Concept expansion: `concept_graph.match_tokens(&tokens)` → `concept_graph.expand(&matched_ids)` → expanded label set
        5. Encode expanded: bundle expanded label HVecs → expanded_hv
        6. Second recall: `singularity.find_similar(&expanded_hv, top_k)` — concept-expanded scores
        7. Merge: `retrieval::hybrid::normalize_scores` on both result sets, build `ScoreBreakdown` per hit
        8. Optional rerank: `reranker.rerank(query_text, &mut hits)` — adjusts ordering, never mutates deterministic scores
        9. Sort by `final_score` descending, truncate to `top_k`
      
      Also:
      - `pub fn memory_packet(singularity: &Singularity, query_text: &str, top_k: usize, reranker: Option<&dyn SemanticReranker>) -> Result<MemoryPacket>`
        Calls `query()` then compiles packet with token-budget compression.
      
      Scoring formula:
        `final_score = 0.6 * deterministic + 0.3 * concept + 0.1 * semantic`
        (weights configurable via BridgeConfig, semantic = 0.0 when no reranker)
      
      Reuse: `retrieval::hybrid::normalize_scores` for score normalization.
      Reuse: `singularity.find_similar` / `find_similar_cached` for both recall passes.

- name: add_memory_grounding_field
  - file: src/singularity.rs (+3 LOC, 491→494)
  - cost: 1
  - description: |
      Add to `Concept` struct:
      ```rust
      #[serde(default)]
      pub canonical_concept_ids: Vec<String>,
      ```
      Backward compatible: existing serialized concepts deserialize with empty vec.
      This links stored memories to canonical concepts for concept-score calculation.

- name: add_builder_canonical_concepts
  - file: src/concept_builder.rs (+10 LOC, 154→164)
  - cost: 1
  - description: |
      Add builder method:
      ```rust
      pub fn with_canonical_concepts(mut self, ids: Vec<String>) -> Self {
          self.canonical_concept_ids = ids;
          self
      }
      ```
      Update `build()` to pass through to `Concept`.

- name: create_framework_bridge
  - file: src/framework_bridge.rs (NEW, ~100 LOC)
  - cost: 3
  - description: |
      Async framework wrappers (keeps framework.rs at 474 LOC, untouched):
      - `impl ChaoticSemanticFramework`
      - `pub async fn probe_bridge_text(&self, query: &str, top_k: usize, bridge: &BridgeRetrieval) -> Result<Vec<BridgeHit>>`
        Acquires singularity read lock, delegates to `bridge.query()`.
      - `pub async fn probe_bridge_text_filtered(&self, query: &str, top_k: usize, bridge: &BridgeRetrieval, filter: &MetadataFilter) -> Result<Vec<BridgeHit>>`
        Same with metadata pre-filtering.
      - `pub async fn memory_packet_text(&self, query: &str, top_k: usize, bridge: &BridgeRetrieval) -> Result<MemoryPacket>`
        Acquires singularity read lock, delegates to `bridge.memory_packet()`.

### Phase 57: Memory Packet & Compression (cost: 6)

```yaml
preconditions:
  bridge_retrieval_pipeline: true
effects:
  memory_packet_output: true
```

- name: implement_packet_compilation
  - file: src/bridge_retrieval.rs (within existing budget, +30-50 LOC)
  - cost: 3
  - description: |
      Add to BridgeRetrieval:
      - `fn compile_packet(query: &str, hits: &[BridgeHit], config: &BridgeConfig) -> MemoryPacket`
      Logic:
        1. Extract `query_intent` from normalized query
        2. For each hit, extract fact from metadata `_text` field or `text_preview`
        3. Deduplicate facts (exact string match)
        4. Truncate to `config.max_packet_facts`
        5. Apply token budget: estimate token count per fact, drop lowest-scored facts until within budget
        6. Collect source IDs
        7. Compute confidence as mean of top-k `final_score` values
      Output matches Issue #52 spec: `{ query_intent, facts, sources, confidence }`

- name: add_packet_serialization
  - file: src/semantic_bridge.rs (+20 LOC within budget)
  - cost: 1
  - description: |
      Add `MemoryPacket::to_json() -> Result<String>` and `MemoryPacket::to_json_pretty() -> Result<String>`.
      Add `MemoryPacket::estimated_tokens() -> usize` (word-count / 0.75 heuristic).

- name: update_export_payload
  - file: src/export_payload.rs (+5 LOC, 156→161)
  - cost: 1
  - description: |
      Include `canonical_concept_ids` in the binary export/import conversion.
      Backward compatible: empty vec for pre-bridge exports.

- name: register_bridge_retrieval_module
  - file: src/lib.rs (+4 LOC within budget)
  - cost: 1
  - description: |
      Add `pub mod bridge_retrieval;` and `pub mod framework_bridge;` to lib.rs.
      Re-export `BridgeRetrieval` from crate root.

### Phase 58: Bridge Persistence (cost: 8, feature-gated)

```yaml
preconditions:
  memory_packet_output: true
effects:
  bridge_persistence: true
```

- name: create_bridge_persistence
  - file: src/bridge_persistence.rs (NEW, ~250 LOC, `#[cfg(feature = "persistence")]`)
  - cost: 6
  - description: |
      Feature-gated persistence for the canonical concept graph:
      - Schema table: `canonical_concepts(id TEXT PK, version INTEGER, labels_json TEXT, related_json TEXT)`
      - `save_concept_graph(persistence: &Persistence, graph: &ConceptGraph) -> Result<()>`
        Batch insert/upsert all concepts in a transaction.
      - `load_concept_graph(persistence: &Persistence) -> Result<ConceptGraph>`
        Load all rows, rebuild label index.
      - `save_canonical_concept(persistence: &Persistence, concept: &CanonicalConcept) -> Result<()>`
      - `delete_canonical_concept(persistence: &Persistence, id: &str) -> Result<()>`
      - Schema migration: add table in `apply_migrations()` at next version increment.
      Uses existing `Persistence` connection model (per-operation from Arc<Database>).

- name: register_bridge_persistence
  - file: src/lib.rs (+2 LOC)
  - cost: 1
  - description: |
      Add `#[cfg(all(not(target_arch = "wasm32"), feature = "persistence"))] pub mod bridge_persistence;`

- name: update_goap_state
  - file: plans/GOAP_STATE.md, plans/ACTIONS.md, plans/ADR_REGISTRY.md
  - cost: 1
  - description: |
      Add Phase 55-58 state variables to GOAP_STATE.md.
      Add Phase 55-58 actions to ACTIONS.md.
      Add ADR-0061 as Accepted to ADR_REGISTRY.md.

### Phase 59: Tests & Benchmarks (cost: 8)

```yaml
preconditions:
  bridge_persistence: true
effects:
  bridge_tests: true
  bridge_benchmarks: true
```

- name: create_concept_graph_tests
  - file: tests/concept_graph.rs (NEW, ~150 LOC)
  - cost: 2
  - description: |
      Test cases:
      - add_concept indexes all labels
      - match_tokens returns correct concept_ids (case-insensitive)
      - expand follows related edges and collects labels
      - expand does not infinite-loop on cycles (A related→B related→A)
      - remove_concept cleans up label_index
      - load_from_json / save_to_json roundtrip
      - empty graph returns empty results
      - duplicate labels across concepts return all matching concept_ids

- name: create_bridge_retrieval_tests
  - file: tests/bridge_retrieval.rs (NEW, ~200 LOC)
  - cost: 3
  - description: |
      Test cases:
      - Full pipeline: query with synonym returns expanded results
      - "agent memory" query finds concepts tagged with "session memory" alias
      - No reranker path: semantic score is 0.0, final_score uses only deterministic+concept
      - ScoreBreakdown values are in [0.0, 1.0]
      - Deterministic scores are never mutated by reranker
      - memory_packet output has correct structure (facts, sources, confidence)
      - Token budget compression drops lowest-scored facts
      - Empty concept graph degrades gracefully to deterministic-only recall
      - WASM-compatible: no persistence or rayon dependencies in bridge core

- name: create_bridge_benchmarks
  - file: benches/benchmark.rs (+40 LOC within budget)
  - cost: 2
  - description: |
      Benchmark groups:
      - `concept_expansion_100_labels`: expand 100-label concept graph
      - `bridge_retrieval_1k_concepts`: full pipeline with 1k stored concepts
      - `memory_packet_compilation`: packet assembly from 20 BridgeHits

- name: create_bridge_persistence_tests
  - file: tests/bridge_persistence.rs (NEW, ~100 LOC, `#[cfg(feature = "persistence")]`)
  - cost: 1
  - description: |
      Test cases:
      - save/load concept graph roundtrip
      - schema migration adds canonical_concepts table
      - canonical_concept_ids field survives concept export/import

---

## File Budget Summary

| File | Current LOC | Δ | Final LOC | Status |
|---|---|---|---|---|
| `src/semantic_bridge.rs` | NEW | +220 | 220 | ✅ |
| `src/concept_graph.rs` | NEW | +300 | 300 | ✅ |
| `src/bridge_retrieval.rs` | NEW | +380 | 380 | ✅ |
| `src/framework_bridge.rs` | NEW | +100 | 100 | ✅ |
| `src/bridge_persistence.rs` | NEW | +250 | 250 | ✅ |
| `src/singularity.rs` | 491 | +3 | 494 | ✅ |
| `src/concept_builder.rs` | 154 | +10 | 164 | ✅ |
| `src/encoder.rs` | 474 | +5 | 479 | ✅ |
| `src/lib.rs` | 201 | +14 | 215 | ✅ |
| `src/export_payload.rs` | 156 | +5 | 161 | ✅ |
| `src/framework.rs` | 474 | +0 | 474 | ✅ untouched |
| `tests/concept_graph.rs` | NEW | +150 | 150 | ✅ |
| `tests/bridge_retrieval.rs` | NEW | +200 | 200 | ✅ |
| `tests/bridge_persistence.rs` | NEW | +100 | 100 | ✅ |

All files under 500 LOC. ✅

---

## Dependency Graph

```
Phase 55 (types + graph)
    ├── accept ADR-0061
    ├── semantic_bridge.rs (types)
    ├── concept_graph.rs (graph)
    ├── encoder.rs (shared tokenizer)
    └── lib.rs (module registration)
        │
        ▼
Phase 56 (retrieval pipeline)
    ├── bridge_retrieval.rs (pipeline)
    ├── singularity.rs (grounding field)
    ├── concept_builder.rs (builder method)
    └── framework_bridge.rs (async wrappers)
        │
        ▼
Phase 57 (memory packet)
    ├── bridge_retrieval.rs (packet compilation)
    ├── semantic_bridge.rs (packet serialization)
    └── export_payload.rs (grounding in export)
        │
        ▼
Phase 58 (persistence)
    ├── bridge_persistence.rs (schema + CRUD)
    └── lib.rs (module registration)
        │
        ▼
Phase 59 (tests + benchmarks)
    ├── tests/concept_graph.rs
    ├── tests/bridge_retrieval.rs
    ├── tests/bridge_persistence.rs
    └── benches/benchmark.rs
```

---

## Validation Gates

After each phase, run:
```bash
cargo check
cargo test --all-features
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
# LOC gate
for file in $(find src -name '*.rs'); do
  LOC=$(wc -l < "$file")
  [ "$LOC" -gt 500 ] && echo "❌ $file exceeds 500 LOC ($LOC)" && exit 1
done
```

---

## Non-Goals

- ❌ Replace deterministic HDC memory with embeddings
- ❌ Ship a default LLM-based reranker (trait only, BYOM)
- ❌ Modify existing `probe()` / `probe_text()` behavior
- ❌ Merge ConceptGraph edges into Singularity associations
- ❌ Add external ML dependencies to the crate

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Normalization drift between encoder and bridge | HIGH | Shared tokenizer extracted from encoder.rs |
| ConceptGraph aliases mixed into associations | HIGH | Separate graph struct, separate persistence table |
| Cache key collision (bridge vs direct probe) | MEDIUM | v1: only cache underlying HDC recalls, not bridge results |
| singularity.rs exceeds 500 LOC | MEDIUM | Only +3 LOC (grounding field), verified at 494 |
| Over-engineering before validating recall improvement | MEDIUM | Phase 55-56 first, measure synonym recall before Phase 57-58 |
