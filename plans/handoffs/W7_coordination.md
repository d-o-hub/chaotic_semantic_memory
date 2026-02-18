# Wave 7: Documentation & Parity Improvements

**Started**: 2026-02-17  
**Status**: Active

## Overview

Wave 7 addresses gaps identified in the analysis swarm review, focusing on:
- Documentation completeness (config structs, README)
- Observability coverage (tracing, metrics)
- Testing pragmatism (focused benchmarks and error tests)
- WASM API parity (temporal processing, memory export/import)

## Groups

### Group A: Testing Pragmatism
**Focus**: Phase 15 actions
- Add to_hypervector benchmark
- Add critical error path tests (3-5 focused cases)

### Group B: Documentation Improvements
**Focus**: Phase 13 actions
- Document FrameworkConfig and SingularityConfig
- Expand README with Installation and Configuration
- Create basic_in_memory example
- Add cargo aliases

### Group C: Observability Completion
**Focus**: Phase 14 actions
- Add tracing to singularity.rs
- Add cache hit/miss metrics
- Add reservoir operation metrics

### Group D: WASM Parity
**Focus**: Phase 16 actions
- Expose process_sequence() to WASM
- Add memory-based export/import (Uint8Array)

## Handoff Contracts

### C -> All
After observability completion, provide tracing span conventions for any new methods added by other groups.

### B -> All
After documentation improvements, provide README template for any new features added.

## ADRs

- ADR-0027: Documentation Standards
- ADR-0028: Observability Completion
- ADR-0029: WASM API Parity

## Success Criteria

1. All Phase 13-16 actions marked complete in GOAP_STATE.md
2. Documentation builds without warnings
3. New benchmarks run successfully
4. WASM binary size remains < 500KB
5. All tests pass including new error path tests
6. Example compiles and runs

## Phase Boundaries

- Phase 13 (Documentation) -> Week 1
- Phase 14 (Observability) -> Week 1-2
- Phase 15 (Testing) -> Week 2
- Phase 16 (WASM) -> Week 2-3

**Target completion**: 2026-02-24
