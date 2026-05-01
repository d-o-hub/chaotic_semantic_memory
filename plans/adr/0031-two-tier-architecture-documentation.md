# ADR-0031: Two-Tier Architecture Documentation

## Status

Accepted (backfilled 2026-05-01)

## Context

Architecture documentation unclear:
- Framework vs Singularity roles not documented
- Two-tier pattern (memory + persistence) not explicit
- Users confused about component boundaries

## Decision

Document **two-tier architecture** explicitly.

**Deliverables:**
- Framework tier: high-level API, async, persistence integration
- Singularity tier: core memory operations, synchronous, in-memory
- Component diagram showing layers
- Architecture chapter in mdBook

## Consequences

### Positive
- Clear mental model for users
- Component boundaries documented
- Decision rationale visible
- Architecture chapter in documentation

### Negative
- Documentation maintenance
- May need updates as architecture evolves

## Implementation

- Files: book/src/architecture.md, README.md
- Diagram: framework -> singularity -> persistence
- Canonical source: context.yaml

## Sources

- ADR_REGISTRY.md: "Two-Tier Architecture Documentation"
- GOAP_STATE.md: architecture_docs_two_tier: true
- book/src/architecture.md