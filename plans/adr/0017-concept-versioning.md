# [ADR-0017] Concept Version History

## Status
Proposed

## Context and Problem Statement
Concepts can be updated over time. Users may need to:
- Audit changes to concept vectors/metadata
- Roll back to previous versions
- Analyze concept evolution
- Debug unexpected behavior

## Decision Drivers
- Audit trail is important for production systems
- Must not significantly impact write performance
- Must be optional (not all use cases need it)
- Storage overhead must be bounded

## Considered Options
1. **No versioning** - Simple, current approach
2. **Event sourcing** - Full history, but complex
3. **Snapshot versioning** - Keep N previous versions
4. **Delta versioning** - Store only changes, efficient but complex

## Decision Outcome
Chosen option: **Snapshot versioning** with configurable retention (default: 10 versions)

### Implementation Strategy
- New table: `concept_versions(concept_id, version, vector, metadata, modified_at)`
- Increment version counter on each update
- Configurable retention policy (default: keep last 10)
- Background cleanup of old versions
- API: `get_concept_history(id, limit) -> Vec<ConceptVersion>`

### Schema
```sql
CREATE TABLE concept_versions (
    concept_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    vector BLOB NOT NULL,
    metadata TEXT NOT NULL,
    modified_at INTEGER NOT NULL,
    PRIMARY KEY (concept_id, version),
    FOREIGN KEY (concept_id) REFERENCES concepts(id) ON DELETE CASCADE
);
```

### Positive Consequences
- Audit trail for compliance/debugging
- Rollback capability
- Bounded storage growth
- Optional feature (opt-in via config)

### Negative Consequences
- Additional write overhead (2x for versioning)
- Additional storage (configurable, but non-zero)
- More complex query patterns

## Links
- [Temporal Tables Pattern](https://en.wikipedia.org/wiki/Temporal_database)
