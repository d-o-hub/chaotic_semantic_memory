# ADR-0017: Concept Versioning

## Status

Accepted (backfilled 2026-05-01)

## Context

Concept modifications lose previous state:
- Original: Update replaces vector/metadata
- Problem: No history for rollback
- Problem: Cannot track evolution

## Decision

Implement **concept version history** with configurable retention.

**Rationale:**
- Track all vector/metadata modifications
- Schema: concept_versions(concept_id, version, vector, modified_at)
- API: get_concept_history(id, limit) -> Vec<ConceptVersion>
- Configurable retention (default: keep last 10)

## Consequences

### Positive
- Version history for rollback
- Track concept evolution
- Configurable retention limits
- Audit trail for modifications

### Negative
- Additional storage overhead
- Version cleanup on delete
- Retention policy complexity

## Implementation

- Module: `src/persistence.rs`, `src/singularity.rs`
- Schema: csm_versions table (v5 prefix)
- Retention: default 10 versions per concept
- API: get_concept_history(id, limit)

## Sources

- ACTIONS.md lines 723-737 (add_concept_versioning action)
- MEMORY.md: "Version retention default: 10 versions per concept"
- src/persistence.rs: version tables