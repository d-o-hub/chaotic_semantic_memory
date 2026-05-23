# Architecture

## Overview

```
┌─────────────────────────────────────────────────────────┐
│                    ChaoticSemanticFramework              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  Singularity │  │  Reservoir  │  │   Persistence   │  │
│  │  (Concepts)  │  │  (Temporal) │  │    (libSQL)     │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │          │
│  ┌──────┴────────────────┴───────────────────┴──────┐   │
│  │                    Hyperdim                       │   │
│  │              (HVec10240 Operations)               │   │
│  └───────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Core Modules

### Hyperdim (`hyperdim.rs`)

10240-bit binary hypervectors with SIMD-accelerated operations:

- `HVec10240` - Dense binary vector type
- `bundle()` - Superposition (XOR + majority)
- `bind()` - Binding (XOR)
- `permute()` - Cyclic shift
- `cosine_similarity()` - Similarity measure

### Reservoir (`reservoir.rs`)

Echo State Network with chaotic dynamics:

- Sparse CSR matrix (fixed degree k=64)
- Spectral radius constraint [0.9, 1.1]
- `step()` - Process single input
- `to_hypervector()` - Project state to HVec10240

### Singularity (`singularity.rs`)

Concept storage and semantic search:

- HashMap-based concept storage
- Association graph (weighted edges)
- Zero-allocation query cache for similarity queries
- Rayon-parallel similarity search

### Framework (`framework.rs`)

High-level async orchestration:

- Builder pattern configuration
- Persistence integration
- Batch operations
- Export/import

## Data Flow

### Concept Injection

```
inject_concept(id, vector)
    → Singularity::inject(id, vector)
    → Persistence::save_concept(id, vector) [if enabled]
```

### Semantic Query

```
probe(vector, top_k)
    → Singularity::find_similar(vector, top_k)
    → Returns Vec<(id, similarity)>
```

### Temporal Processing

```
process_sequence(inputs)
    → Reservoir::reset()
    → Reservoir::step(input) for each input
    → Reservoir::to_hypervector()
    → Returns HVec10240
```

## Constraints

| Constraint | Value | Rationale |
|------------|-------|-----------|
| LOC per file | ≤500 | Maintainability |
| Spectral radius | [0.9, 1.1] | Edge of chaos |
| WASM threading | Gated | Browser compatibility |
| Mutation testing | scripts/mutation_test.sh | Prevent regressions |
| Fuzz testing | cargo +nightly fuzz run hvec_from_bytes | Prevent regressions |
