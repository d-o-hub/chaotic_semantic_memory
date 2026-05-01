# ADR-0026: Namespace Isolation

## Status

Deferred (backfilled 2026-05-01)

## Context

Multi-tenant deployments need namespace separation:
- Problem: All concepts share single namespace
- Problem: No tenant isolation
- Use case: SaaS deployment, multi-user systems

## Decision

**Deferred** - Namespace isolation not implemented for 1.0.

**Rationale:**
- Single-tenant is current use case
- No SaaS deployment requirement
- Namespace adds API complexity
- Complexity not justified for initial release

**Activation Trigger:**
- Multi-tenant SaaS deployment requirements
- User requests for tenant separation
- Enterprise deployment needs

## Consequences

### Positive (Deferred)
- Simple API (no namespace parameter)
- No tenant management overhead
- Easier development and testing

### Negative (Deferred)
- Cannot isolate tenant data
- Single namespace for all concepts
- Requires separate instances for multi-tenant

## Future Implementation

- Module: `src/singularity.rs`, `src/persistence.rs`
- Pattern: namespace prefix on concept IDs
- Schema: namespace column in tables
- Cost: ~10 (estimated)

## Sources

- ACTIONS.md lines 1182-1192 (deferred_namespace_isolation action)
- ADR_REGISTRY.md: Deferred ADRs section
- Note: ADR-0073 exists for similar concept