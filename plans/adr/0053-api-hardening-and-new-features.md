# ADR-0053: API Hardening & New Features

## Status

Proposed (backfilled 2026-05-01) - Wave 15

## Context

API gaps and feature requests:
- Missing API methods for common use cases
- Error handling gaps
- Input validation incomplete
- API documentation gaps

## Decision

Propose **API hardening & new features**.

**Proposed Scope:**
- Additional API methods for common patterns
- Error handling improvements
- Input validation completion
- API documentation expansion
- Type safety improvements

## Consequences

### Positive
- Complete API coverage
- Better error handling
- Validated inputs throughout
- Clear API documentation

### Negative
- API expansion complexity
- Breaking changes possible
- Documentation maintenance
- Feature scope unclear

## Implementation

- Module: src/framework.rs, src/singularity.rs
- Phase: 32-36 (Wave 15)
- Depends on: Wave 14 completion

## Sources

- ADR_REGISTRY.md: API Hardening & New Features (Proposed)
- ACTIONS.md lines 2017-2199 (ADR-0053 actions)
- Wave 15 planning