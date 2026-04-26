# Semantic Bridge

`SemanticBridge` provides symbolic concept expansion on top of deterministic HDC recall, combining concept-graph traversal with vector similarity for hybrid retrieval.

## Why It Exists

- Expands queries through synonym and related-concept graphs without mutating stored vectors
- Combines deterministic HDC scores with concept expansion scores for richer recall
- Produces compressed `MemoryPacket` structs ready for LLM context injection
- Optional semantic reranking layer that never modifies deterministic scores

## Core Types

### `CanonicalConcept`

Represents a semantic equivalence class (e.g., "agent memory" ≈ "cross-session context"):

```rust,no_run
use chaotic_semantic_memory::semantic_bridge::CanonicalConcept;

let concept = CanonicalConcept::new("concept.agent_memory")
    .with_label("agent memory")
    .with_label("cross-session context")
    .with_related("concept.session_state");
```

### `ConceptGraph`

In-memory graph of canonical concepts with case-insensitive label matching:

```rust,no_run
use chaotic_semantic_memory::semantic_bridge::{CanonicalConcept, ConceptGraph};

let mut graph = ConceptGraph::new();

graph.add_concept(
    CanonicalConcept::new("concept.agent_memory")
        .with_label("agent memory")
        .with_label("ai memory")
        .with_related("concept.session"),
);
graph.add_concept(
    CanonicalConcept::new("concept.session")
        .with_label("session context")
        .with_label("cross-session state"),
);

// Case-insensitive token matching
let ids = graph.match_tokens(&["Agent-Memory".into()]);

// Expand through related concepts (depth=2)
let labels = graph.expand(&ids, 2);
```

The graph supports JSON serialization for persistence:

```rust,no_run
# use chaotic_semantic_memory::semantic_bridge::ConceptGraph;
# let graph = ConceptGraph::new();
// Save
let file = std::fs::File::create("concepts.json").unwrap();
graph.save_to_json(file).unwrap();

// Load
let file = std::fs::File::open("concepts.json").unwrap();
let graph = ConceptGraph::load_from_json(file).unwrap();
```

### `BridgeConfig`

Controls retrieval behavior and score weighting:

```rust,no_run
use chaotic_semantic_memory::semantic_bridge::BridgeConfig;

let config = BridgeConfig {
    max_expansion_depth: 2,    // How deep to traverse related concepts
    max_packet_facts: 20,      // Max facts in a MemoryPacket
    token_budget: 1000,        // Token cap for packet compression
    deterministic_weight: 0.6, // HDC similarity weight
    concept_weight: 0.3,       // Concept expansion weight
    semantic_weight: 0.1,      // Reranker weight
};
```

Default weights sum to 1.0: deterministic (0.6), concept (0.3), semantic (0.1).

## Query Pipeline

`BridgeRetrieval` orchestrates the full retrieval pipeline:

```text
Query text
  │
  ├─► Tokenize & encode ──► HDC similarity (deterministic recall)
  │
  ├─► Token matching ──► Concept expansion ──► Encode expanded labels
  │                                               │
  │                                               └─► HDC similarity (expanded recall)
  │
  ├─► Merge results with ScoreBreakdown
  │
  ├─► Optional SemanticReranker
  │
  └─► Final weighted score ──► Top-K hits
```

### Basic Query

```rust,no_run
use chaotic_semantic_memory::encoder::TextEncoder;
use chaotic_semantic_memory::semantic_bridge::{
    BridgeConfig, CanonicalConcept, ConceptGraph,
};
use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
use chaotic_semantic_memory::prelude::{ConceptBuilder, Result};
use chaotic_semantic_memory::singularity::Singularity;

fn main() -> Result<()> {
    let encoder = TextEncoder::new();

    // Build concept graph
    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("concept.memory")
            .with_label("agent memory")
            .with_label("session context")
            .with_related("concept.persistence"),
    );
    graph.add_concept(
        CanonicalConcept::new("concept.persistence")
            .with_label("durable storage")
            .with_label("database"),
    );

    // Create retrieval pipeline
    let bridge = BridgeRetrieval::with_defaults(encoder.clone(), graph);

    // Populate singularity
    let mut singularity = Singularity::new();
    singularity.inject(
        ConceptBuilder::new("doc-1")
            .with_vector(encoder.encode("agent memory across sessions"))
            .build()?,
    )?;
    singularity.inject(
        ConceptBuilder::new("doc-2")
            .with_vector(encoder.encode("durable storage for AI state"))
            .build()?,
    )?;

    // Query: deterministic + concept-expanded recall
    let hits = bridge.query(&singularity, "session context", 5, None)?;
    for hit in &hits {
        println!("{}: {:.3} (det={:.3}, concept={:.3})",
            hit.id,
            hit.scores.final_score,
            hit.scores.deterministic,
            hit.scores.concept,
        );
    }
    Ok(())
}
```

### Memory Packets

`memory_packet()` runs the query pipeline and compresses results into a token-budgeted packet for LLM context:

```rust,no_run
# use chaotic_semantic_memory::encoder::TextEncoder;
# use chaotic_semantic_memory::semantic_bridge::ConceptGraph;
# use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
# use chaotic_semantic_memory::singularity::Singularity;
# let encoder = TextEncoder::new();
# let bridge = BridgeRetrieval::with_defaults(encoder, ConceptGraph::new());
# let singularity = Singularity::new();
let packet = bridge.memory_packet(&singularity, "agent memory", 10, None).unwrap();

println!("Intent: {}", packet.query_intent);
println!("Facts: {:?}", packet.facts);
println!("Confidence: {:.2}", packet.confidence);
println!("Est. tokens: {}", packet.estimated_tokens());
```

## Result Types

### `BridgeHit`

Each hit carries a full `ScoreBreakdown`:

| Field | Description |
|-------|-------------|
| `id` | Concept ID in the singularity |
| `text_preview` | Optional text snippet |
| `scores.deterministic` | HDC cosine similarity score |
| `scores.concept` | Concept expansion match score |
| `scores.semantic` | Reranker score (0.0 if unused) |
| `scores.final_score` | Weighted combination of all scores |
| `scores.evidence` | Trail of scoring sources (e.g., `"deterministic_recall"`, `"concept_expansion"`) |

### `MemoryPacket`

Compressed output for LLM context injection:

| Field | Description |
|-------|-------------|
| `query_intent` | Original query text |
| `facts` | Deduplicated, token-budgeted fact strings |
| `sources` | Source concept IDs |
| `confidence` | Average final score across top hits |

Supports `to_json()` and `to_json_pretty()` for serialization.

## Custom Reranking

Implement `SemanticReranker` to plug in model-based or rule-based reranking:

```rust,no_run
use chaotic_semantic_memory::semantic_bridge::{BridgeHit, SemanticReranker};

struct MyReranker;

impl SemanticReranker for MyReranker {
    fn version(&self) -> &str { "custom-v1" }

    fn rerank(&self, query: &str, hits: &mut [BridgeHit]) {
        for hit in hits {
            // Boost hits whose ID contains a query token
            let query_lower = query.to_lowercase();
            if hit.id.to_lowercase().contains(&query_lower) {
                hit.scores.semantic = 0.5;
            }
        }
    }
}
```

Pass it to `query()` or `memory_packet()`:

```rust,no_run
# use chaotic_semantic_memory::bridge_retrieval::BridgeRetrieval;
# use chaotic_semantic_memory::singularity::Singularity;
# struct MyReranker;
# impl chaotic_semantic_memory::semantic_bridge::SemanticReranker for MyReranker {
#     fn version(&self) -> &str { "v1" }
#     fn rerank(&self, _q: &str, _h: &mut [chaotic_semantic_memory::semantic_bridge::BridgeHit]) {}
# }
# let bridge = BridgeRetrieval::with_defaults(
#     chaotic_semantic_memory::encoder::TextEncoder::new(),
#     chaotic_semantic_memory::semantic_bridge::ConceptGraph::new(),
# );
# let singularity = Singularity::new();
let reranker = MyReranker;
let hits = bridge.query(&singularity, "agent memory", 5, Some(&reranker)).unwrap();
```

## Score Weighting

Final scores are computed as:

```text
final = deterministic_weight × deterministic
      + concept_weight      × concept
      + semantic_weight     × semantic
```

Tune via `BridgeConfig` to control the balance between exact HDC recall and concept expansion. Higher `deterministic_weight` favors precision; higher `concept_weight` favors broader semantic recall.
