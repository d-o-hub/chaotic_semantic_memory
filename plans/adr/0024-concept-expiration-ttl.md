# ADR-0024: Concept Expiration (TTL) Baseline

## Status

Implemented (baseline and advanced policies; cleanup-task lifecycle follow-up proposed in ADR-0093)

## Context

Memory concepts may need time-based expiration:
- Problem: Concepts accumulate indefinitely
- Problem: No cleanup mechanism for stale data
- Use case: Session-based memory, temporary concepts

## Decision

Implement baseline TTL APIs and advanced policy automation.

**Baseline (Implemented):**
- inject_concept_with_ttl(id, vector, metadata, ttl_seconds)
- inject_text_with_ttl(text, ttl_seconds)
- purge_expired() -> removes all expired concepts
- expires_at field in persistence

**Advanced policies (Implemented 2026-06-23):**
- Fixed, MetadataRule, and Inherit TTL policies
- Cascading purge and DecayCurve behavior
- Opt-in background cleanup scheduling

**Lifecycle follow-up:**
- The background task exists, but cancellation/JoinHandle ownership and bounded shutdown are proposed in ADR-0093.

## Consequences

### Positive
- Basic TTL capability available
- Explicit API for time-bounded concepts
- Manual purge control
- Activation trigger: session management use case

### Negative
- No automatic cleanup (manual purge_required)
- Deferred features require future development
- TTL tracking overhead

## Implementation

- Module: `src/framework_ttl.rs`, `src/singularity_ttl.rs`
- API: inject_with_ttl, purge_expired
- Persistence: expires_at column
- Activation: >50% concepts are ephemeral use case

## Sources

- ACTIONS.md lines 1131-1168 (implement_concept_ttl_baseline, deferred_concept_ttl)
- MEMORY.md: TTL support documented
- src/framework_ttl.rs: TTL implementation