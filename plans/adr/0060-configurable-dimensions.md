# ADR-0060: Configurable Hypervector Dimensions

## Status
Deferred

## Context
Issue #35 raised the concern that 10,240 dimensions for hypervectors is excessive:
- Each vector is ~10KB (1280 bytes)
- Even aggressive HDC papers rarely exceed 4,096 for text
- Sentence-transformers use 384-768 dimensions
- Memory and compute costs are 2.5× what's potentially needed

## Decision
Defer configurable dimensions to a future release (post-1.0).

## Rationale
1. **Breaking Change**: Making HVec10240 generic over size would be a breaking API change
2. **Widespread Impact**: HVec10240 is used throughout the codebase:
   - singularity.rs, framework.rs, persistence.rs
   - WASM bindings, CLI, examples
   - All would need updates
3. **Current Mitigation**: Users can:
   - Use `default-features = false` for minimal dependencies
   - Bundle multiple smaller concepts to reduce individual vector size
   - Use external embedding models with smaller dimensions
4. **Trigger Condition**: Implement when benchmarks demonstrate memory pressure
   at scale (>200k concepts) or when users specifically request this feature

## Alternatives Considered
1. **Const Generics**: `HVec<const N: usize>` - Requires Rust 1.51+ and significant refactoring
2. **Multiple Types**: `HVec1024`, `HVec2048`, `HVec4096`, `HVec10240` - Code duplication
3. **Runtime Configuration**: Store dimension in struct - Performance impact, not zero-cost

## Consequences
- Users with strict memory constraints should use external embeddings
- This decision will be revisited if user feedback indicates strong need
- Document current dimension (10,240) clearly in README