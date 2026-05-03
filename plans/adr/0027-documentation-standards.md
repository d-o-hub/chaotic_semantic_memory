# ADR-0027: Documentation Standards

## Status

Accepted (backfilled 2026-05-01) - Wave 7 Complete

## Context

Documentation gaps identified:
- FrameworkConfig and SingularityConfig undocumented
- README missing installation/configuration sections
- No basic usage example
- Cargo aliases not documented

## Decision

Implement **comprehensive documentation standards**.

**Deliverables:**
- rustdocs on FrameworkConfig/SingularityConfig
- README: Installation, Configuration, API Patterns sections
- examples/basic_in_memory.rs: minimal working example
- .cargo/config.toml: developer aliases

## Consequences

### Positive
- Users can discover configuration options
- Clear installation instructions
- Runnable example for new users
- Developer aliases improve DX

### Negative
- Documentation maintenance overhead
- Rustdocs require keeping docs in sync with code
- Example must be kept working

## Implementation

- Files: src/framework.rs, src/singularity.rs, README.md
- Example: examples/basic_in_memory.rs (<100 LOC)
- Aliases: test-all, bench-local, check-wasm, fmt-check

## Sources

- ACTIONS.md lines 924-996 (Phase 13 actions)
- ADR_REGISTRY.md: Wave 7 Active ADRs
- Git: documentation commits