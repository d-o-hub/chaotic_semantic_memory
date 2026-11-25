# ADR-0026: Namespace Isolation

## Status

Implemented (namespace mechanism, resolved 2026-11-25); multi-tenant isolation deferred

## Context

Multi-tenant deployments need namespace separation:
- Problem: All concepts share single namespace
- Problem: No tenant isolation
- Use case: SaaS deployment, multi-user systems

## Decision

**Deferred → partially resolved (2026-11-25 truth-up, issue #1064).** The original
1.0 decision deferred the whole feature (single-tenant use case, API complexity,
complexity not justified at the time). What actually shipped in 0.3.x is the
namespace *mechanism*: one active namespace per framework instance
(`set_namespace`, `src/framework_namespaces.rs`), namespace-parameterized
persistence APIs (`save_concept_ns` and friends), and per-namespace durable
tables plus revisions (namespace- and revision-scoped absence records,
`csm_namespace_meta`). The registry row correctly says **Implemented** for that
mechanism.

**Multi-tenant isolation semantics remain deferred**: there is no per-tenant
authorization boundary, and the active namespace is instance-global — switching
it affects every user of that instance. `GOAP_STATE.deferred_namespace_isolation`
stays false until that lands.

**Activation Trigger (unmet):**
- Multi-tenant SaaS deployment requirements
- User requests for tenant separation
- Enterprise deployment needs

## Consequences

### Positive (mechanism, implemented)
- Namespaced durable tables and per-namespace revisions isolate data on disk
- One instance per tenant namespace is a workable deployment shape today

### Negative (isolation, deferred)
- No per-tenant authorization boundary
- Active namespace is instance-global; `set_namespace` is not tenant-switchable
- Concurrent tenants need separate instances (unchanged from the 1.0 posture)

## Future Implementation

- Module: `src/singularity.rs`, `src/persistence.rs`
- Pattern: namespace prefix on concept IDs
- Schema: namespace column in tables
- Cost: ~10 (estimated)

## Sources

- ACTIONS.md lines 1182-1192 (deferred_namespace_isolation action)
- ADR_REGISTRY.md: Deferred ADRs section
- Note: ADR-0073 exists for a similar concept
