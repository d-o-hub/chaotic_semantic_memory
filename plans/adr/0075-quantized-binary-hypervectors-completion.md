# ADR-0075: Quantized Binary Hypervectors Completion

## Context
Wave 24 P3 (ADR-0075) introduced a 32x compressed binary hypervector format (`BHVec10240`) and refactored the framework to be generic over the hypervector representation `<H: Hypervector>`. While the core data structures and persistence migrations are complete, the large-scale refactor introduced significant technical debt and type mismatches in extension modules (GraphRAG, Semantic Bridge, Rerankers) and the CLI.

## Decision
We will complete the generic transition and binary hypervector support in a follow-up task. The remaining work focuses on stabilizing the type system, restoring CLI functionality, and performance optimization.

## Status
In Progress (Partial generic transition complete; extension modules and CLI require stabilization).

## Consequences
- **Type Safety**: The framework will gain compile-time enforcement of hypervector representation consistency.
- **Scalability**: Once completed, the system will support million-scale concept storage on low-memory devices.
- **Maintainability**: The codebase will be more modular but slightly more complex due to pervasive generics.
- **Performance**: SIMD optimizations for binary superposition (bundling) are required to match the performance of the float-based baseline.
