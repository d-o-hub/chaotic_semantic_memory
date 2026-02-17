# ADR-0027: Documentation Standards for Public API

## Status
- **Proposed**: 2026-02-17
- **Accepted**: Pending

## Context

The chaotic_semantic_memory crate has comprehensive functionality but inconsistent documentation coverage. Analysis revealed that `FrameworkConfig` and `SingularityConfig` structs lack rustdocs for their fields, making it difficult for users to understand configuration options. The README also lacks Installation and Configuration sections that users expect.

This ADR establishes documentation standards to improve developer experience without adding maintenance burden.

## Decision

### 1. Public API Documentation Requirements

All public structs, methods, and functions must have:
- **Brief description**: One-sentence summary
- **Usage example**: When non-obvious (optional for trivial getters)
- **Default values**: For all configuration options
- **Valid ranges**: For numeric parameters

### 2. Configuration Struct Documentation Pattern

```rust
/// Configuration for [ChaoticSemanticFramework].
///
/// # Examples
///
/// ```
/// use chaotic_semantic_memory::FrameworkConfig;
///
/// let config = FrameworkConfig {
///     max_concepts: 100_000,
///     ..Default::default()
/// };
/// ```
///
/// # Field Documentation
///
/// - `max_concepts`: Maximum number of concepts before eviction (default: 1,000,000)
/// - `max_associations_per_concept`: Association limit per concept (default: 1000)
/// - `cache_size`: LRU cache size for similarity queries (default: 1000)
pub struct FrameworkConfig {
    pub max_concepts: usize,
    pub max_associations_per_concept: usize,
    pub cache_size: usize,
}
```

### 3. README Structure

The README must include:
1. **Installation**: `cargo add` command with feature flags
2. **Quick Start**: Minimal working example (current)
3. **Configuration**: Parameter table with descriptions
4. **API Patterns**: Common usage patterns
5. **Architecture**: Brief overview (current)

### 4. Example Code Requirements

Examples must:
- Compile and run successfully
- Demonstrate one clear concept each
- Be under 150 lines
- Include `cargo run --example <name>` instructions

## Consequences

### Positive
- Reduced user confusion about configuration options
- Faster onboarding for new users
- Fewer "how do I..." support requests
- Better IDE autocomplete experience

### Negative
- Documentation can become stale if not updated with code
- Requires discipline to maintain
- Adds ~200-300 lines to source files

### Migration Path
- Phase 1: Document config structs (immediate)
- Phase 2: Expand README sections (Week 1)
- Phase 3: Add minimal examples (Week 1)
- Phase 4: Add cargo aliases (Week 1)

## Compliance

- **AGENTS.md LOC limit**: Yes, additions are documentation only
- **No hardcoded settings**: Documentation references constants, doesn't add them
- **WASM compatible**: No code changes, only docs

## References

- Analysis artifact: `plans/handoffs/analysis_group_c_docs.md`
- Current README: `README.md`
- Target files: `src/framework.rs`, `src/singularity.rs`
