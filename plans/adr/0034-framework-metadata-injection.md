# [ADR-0034] Framework Metadata Injection and WASM Batch API Parity

## Status
Proposed

## Context and Problem Statement
Two API completeness gaps exist:

1. **No metadata injection at framework level**: `ChaoticSemanticFramework::inject_concept()` only accepts `(id, vector)`. The underlying `ConceptBuilder` supports metadata, and `FrameworkConfig` supports `max_metadata_bytes` validation, but there is no public API path that connects them. Users must bypass the framework to inject concepts with metadata.

2. **WASM API parity gaps**: The WASM `WasmFramework` wrapper is missing several APIs that exist on the native framework:
   - `get_concept()` — retrieve a concept by ID
   - `inject_concepts()` — batch inject
   - `associate_many()` — batch associate
   - `probe_batch()` — batch probe
   - `bundle_concepts()` — bundle operation

3. **Missing builder setter**: `FrameworkBuilder` has no `with_reservoir_input_size()` method, preventing users from configuring the input dimension for `process_sequence()`.

## Decision Drivers
- Metadata is a first-class concept property but unreachable via the recommended API path
- WASM users lack batch APIs available to native users
- Builder completeness is a basic API contract

## Considered Options
- Option A: Add `inject_concept_with_metadata()` method + WASM batch APIs + builder setter
- Option B: Change `inject_concept()` to accept an optional metadata parameter
- Option C: Add `inject(Concept)` that accepts a pre-built concept

## Decision Outcome
Chosen option: "Option A + C hybrid", because it preserves backward compatibility while adding the missing surfaces.

### Implementation
1. Add `inject_concept_with_metadata(id, vector, metadata)` on `ChaoticSemanticFramework` — validates `max_metadata_bytes` and passes through to `ConceptBuilder`
2. Add `with_reservoir_input_size(size)` on `FrameworkBuilder`
3. Add WASM bindings for: `get_concept`, `inject_concepts`, `associate_many`, `probe_batch`
4. Remove unused `--config` flag from CLI (or implement it)

### Positive Consequences
- Complete metadata lifecycle (create with metadata → query → filter)
- WASM feature parity with native API
- Full builder coverage for all config fields

### Negative Consequences
- Additional methods increase API surface (minor maintenance cost)
- WASM bindings file grows (~40 lines, still well under 500 LOC)
