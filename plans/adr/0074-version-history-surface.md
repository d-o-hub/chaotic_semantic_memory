# ADR-0074: Concept Version History Surface

## Status

Proposed (2026-04-30)

## Context and Problem Statement

`src/persistence_versions.rs` records concept version history (vector + metadata at each update). The data is captured but invisible to users:
- No CLI command lists, diffs, or rolls back versions
- No WASM API exposes version history
- No Framework method retrieves prior versions

Users can update concepts but cannot inspect or recover prior states. This makes the version table a write-only audit log with no read path.

## Decision Drivers

- Reuse existing schema (`concept_versions` table)
- Backward compatible
- Append-only (no destructive ops on versions table)
- LOC budget ≤ 350/file

## Considered Options

1. **Add list/get/rollback API** at Framework + CLI + WASM
2. Add only list API
3. Remove version table (it's unused at the surface)

## Decision Outcome

Chosen: **Option 1** — full surface. Versions exist for a reason; expose them.

## Implementation

### Framework API

```rust
impl Framework {
    pub async fn list_versions(&self, id: &str) -> Result<Vec<ConceptVersion>>;
    pub async fn get_version(&self, id: &str, version: u64) -> Result<Option<Concept>>;
    pub async fn diff_versions(
        &self,
        id: &str,
        from: u64,
        to: u64,
    ) -> Result<ConceptDiff>;
    pub async fn rollback_to_version(&self, id: &str, version: u64) -> Result<()>;
}

pub struct ConceptVersion {
    pub version: u64,
    pub timestamp_unix: i64,
    pub vector_changed: bool,
    pub metadata_changed: bool,
}

pub struct ConceptDiff {
    pub vector_cosine_distance: f32,
    pub metadata_added: HashMap<String, JsonValue>,
    pub metadata_removed: HashMap<String, JsonValue>,
    pub metadata_changed: HashMap<String, (JsonValue, JsonValue)>,
}
```

### CLI

```
csm history <id>                    # list versions
csm get <id> --version 3            # fetch a specific version
csm diff <id> --from 2 --to 5       # diff two versions
csm rollback <id> --to 3 --confirm  # restore to version 3 (creates new version)
```

### WASM

```typescript
framework.listVersions(id): Promise<ConceptVersion[]>
framework.getVersion(id, version): Promise<Concept | null>
framework.rollbackToVersion(id, version): Promise<void>
```

### Implementation notes

- `rollback` creates a NEW version that mirrors the target — never destructive on history
- `diff` uses `vector_cosine_distance` rather than byte equality (vectors are normalized)
- Pagination on `list_versions` (default limit 100, configurable)
- Respects existing `version_retention` cap (no surprise data growth)

## Pros and Cons

### Pros
- Activates existing dormant data
- Crucial for memory systems where "what did the agent know yesterday" matters
- Symmetric with how `git log`/`git checkout` work

### Cons
- New CLI commands to maintain
- Diff semantics need careful documentation
- Rollback creates a new version — must communicate clearly

## Acceptance Criteria

- [ ] All 4 Framework methods implemented
- [ ] 4 CLI subcommands work
- [ ] WASM bindings exposed
- [ ] `tests/version_history.rs` covers list/get/diff/rollback
- [ ] Documentation page in book
- [ ] Per-file LOC ≤ 350
