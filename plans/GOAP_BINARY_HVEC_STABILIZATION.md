# GOAP: Quantized Binary Hypervectors Stabilization

## Overview
This plan addresses the technical debt and type-inference issues introduced during the generic transition of ADR-0075.

## State
- **Core**: `Hypervector` trait, `HVec10240` (f32), and `BHVec10240` (binary) are implemented.
- **Storage**: `Singularity<H>` and `Concept<H>` are generic.
- **Persistence**: Migration v9 is implemented; polymorphic loading is functional.
- **Extensions**: GraphRAG, Semantic Bridge, and Rerankers have type mismatches and broken imports.
- **CLI**: The CLI tool is currently non-functional due to broken re-exports and generic parameter mismatches.

## Goals
1.  **Extension Stabilization**: Restore functionality to GraphRAG and Semantic Bridge with the new generic architecture.
2.  **CLI Restoration**: Fix the `csm` binary and its subcommands to work with the generic framework (defaulting to f32).
3.  **Performance Optimization**: Implement bit-sliced bundling for binary hypervectors to avoid heap allocations.
4.  **HNSW Safety**: Audit the HNSW index for any remaining lifetime issues related to generic `H`.

## Tasks
- [ ] Refactor `src/retrieval/graph_rag.rs` to be fully compatible with `Hypervector` trait methods.
- [ ] Align `src/semantic_bridge.rs` and `src/bridge_retrieval.rs` with the generic `Singularity<H>`.
- [ ] Restore `src/cli/commands/` subcommands (inject, query, etc.) to a functional state.
- [ ] Implement optimized `BHVec10240::bundle` using bitwise logic.
- [ ] Add `Recall@10` benchmark comparing float vs binary formats.
