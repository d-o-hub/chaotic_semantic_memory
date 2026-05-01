# ADR-0034: Framework Metadata Injection

## Status

Accepted (backfilled 2026-05-01) - Wave 10 Complete

## Context

Metadata injection API gaps:
- No inject_concept_with_metadata() API
- No reservoir input size configuration
- WASM missing batch API methods

## Decision

Implement **framework metadata injection and WASM batch parity**.

**Deliverables:**
- inject_concept_with_metadata(id, vector, metadata) on Framework
- with_reservoir_input_size(size) on FrameworkBuilder
- WASM bindings: get_concept, inject_concepts, associate_many, probe_batch

## Consequences

### Positive
- Direct metadata injection without ConceptBuilder
- Configurable reservoir input dimension
- WASM batch API parity with native
- Simpler API for metadata injection

### Negative
- Additional API surface
- WASM batch complexity
- Requires metadata validation

## Implementation

- Module: src/framework.rs, src/wasm.rs
- API: inject_concept_with_metadata
- WASM: batch methods with JsValue conversion

## Sources

- ACTIONS.md lines 1303-1341 (Phase 21 actions)
- ADR_REGISTRY.md: Framework Metadata Injection
- src/framework.rs: inject_with_metadata