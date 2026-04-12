# [ADR-0051] Real-World Readiness & Quality Hardening

## Status
Accepted

## Context and Problem Statement
Analysis swarm identified several actionable improvements to make the crate
production-ready for real-world AI agent and RAG workloads:

1. **Bug: `max_cached_top_k` not propagated** – `FrameworkBuilder::build()` hard-codes
   `DEFAULT_MAX_CACHED_TOP_K` (10 000) instead of forwarding the user-configured value,
   silently ignoring `with_max_cached_top_k()`.
2. **Default mismatch** – `FrameworkBuilder` defaults `max_cached_top_k` to 10 000 while
   `SingularityConfig` defaults to 100. The builder should match Singularity's conservative
   default.
3. **Semantic error misuse** – `Singularity::update()` returns
   `MemoryError::Persistence("Concept not found")` when the concept doesn't exist;
   a dedicated `NotFound` variant is clearer and easier to match.
4. **JSON import unbounded** – `import_json` reads the entire file without size limits,
   unlike `import_binary` which caps at 100 MB.
5. **Missing real-world examples** – Only trivial `basic_in_memory` and `proof_of_concept`
   exist; no chatbot memory, RAG, knowledge graph, or temporal/streaming examples.
6. **Edge-case test gaps** – Builder config propagation, import adversarial payloads,
   eviction + cache invalidation, and concurrent probe+inject under load are untested.

## Decision Drivers
- Production users need correct config propagation and bounded resource usage.
- Real-life examples are the primary onboarding path for new adopters.
- Edge-case tests protect against regressions in critical paths.
- All changes must stay within 500 LOC per file.

## Considered Options
1. Fix bugs only, defer examples and tests.
2. Full wave: bugs + examples + tests (chosen).
3. Defer to post-1.0 milestone.

## Decision Outcome
Chosen option: **2 – Full wave**, because the fixes are low-risk, the examples
significantly improve DX, and the tests catch real bugs already present.

### Positive Consequences
- `with_max_cached_top_k()` actually controls cache behavior.
- JSON import bounded to 100 MB, preventing OOM.
- `NotFound` variant enables clean error matching in application code.
- 4 production-grade examples demonstrate real-world patterns.
- Edge-case tests prevent regression on config, import, eviction, and concurrency.

### Negative Consequences
- `NotFound` variant is a minor breaking change for exhaustive match users.
- Additional examples add maintenance surface.

## Implementation Plan

### Phase 29: Bug Fixes & API Improvements (cost: 6)
| Action | File | Description |
|--------|------|-------------|
| fix_cached_top_k_propagation | framework_builder.rs | Use `self.config.max_cached_top_k` |
| align_default_max_cached_top_k | framework_builder.rs | Change default to 100 |
| add_not_found_error_variant | error.rs, singularity.rs | New `NotFound` variant |
| add_json_import_size_limit | framework_ops.rs | Cap at MAX_IMPORT_SIZE |

### Phase 30: Real-Life Usage Examples (cost: 8)
| Action | File | Description |
|--------|------|-------------|
| chatbot_session_memory | examples/chatbot_memory.rs | Session-based memory |
| document_rag_chunks | examples/document_rag.rs | RAG chunk store |
| knowledge_graph | examples/knowledge_graph.rs | Entity graph traversal |
| streaming_temporal | examples/streaming_temporal.rs | Reservoir + time-series |

### Phase 31: Edge Case Tests (cost: 5)
| Action | File | Description |
|--------|------|-------------|
| builder_config_propagation | tests/builder_config.rs | Verify all builder setters |
| import_adversarial | tests/import_adversarial.rs | Oversized, corrupt, empty |
| eviction_cache_invalidation | tests/edge_case_coverage.rs | Eviction clears cache |
| concurrent_stress | tests/framework_lifecycle.rs | Inject+probe under load |
