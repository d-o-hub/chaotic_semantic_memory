# ADR-0073: Namespace Isolation / Multi-tenancy

## Status

Proposed (2026-04-30) — supersedes deferred ADR-0026

## Context and Problem Statement

All concepts share one keyspace. Cannot host multiple users / projects / agents in one DB without manual prefix collisions. The deferred ADR-0026 was scoped to "SaaS deployment" but the same problem affects:

- Skill-memory ingesting context for multiple repositories
- A single MCP server (ADR-0067) serving multiple clients
- Per-user chat memory in a shared deployment

We need first-class namespace support.

## Decision Drivers

- Backward compatible — existing single-tenant DBs keep working
- Zero overhead for users who don't use namespaces
- Namespace must be queryable, deletable, exportable as a unit
- LOC budget ≤ 500/file
- WASM compatible

## Considered Options

1. **Namespace column + index** — single DB, namespace-tagged rows, all queries filter by namespace
2. Separate DB per namespace
3. Schema-per-namespace (libSQL ATTACH)
4. Prefix-only convention (existing user workaround)

## Decision Outcome

Chosen: **Option 1** — namespace column. Lowest operational cost, supports cross-namespace queries, single backup, single migration path.

## Implementation

### Schema migration `006_add_namespace.sql`

```sql
ALTER TABLE concepts ADD COLUMN namespace TEXT NOT NULL DEFAULT '_default';
ALTER TABLE associations ADD COLUMN namespace TEXT NOT NULL DEFAULT '_default';
CREATE INDEX idx_concepts_namespace ON concepts(namespace);
CREATE INDEX idx_assocs_namespace ON associations(namespace);
```

### Framework API

```rust
impl FrameworkBuilder {
    pub fn with_namespace(self, ns: impl Into<String>) -> Self;
}

impl Framework {
    pub fn namespace(&self) -> &str;
    pub async fn list_namespaces(&self) -> Result<Vec<String>>;
    pub async fn delete_namespace(&self, ns: &str) -> Result<usize>; // returns deleted count
    pub async fn export_namespace(&self, ns: &str, path: &Path) -> Result<()>;
}
```

All existing methods (`inject_concept`, `probe`, `associate`, etc.) automatically filter by `self.namespace`.

### Singularity in-memory layer

```rust
pub struct Singularity {
    namespaces: HashMap<String, NamespaceState>,
    active_namespace: String,
}
```

Hot path stays single-namespace lookup → no perf regression.

### CLI

```
csm --namespace "user_42" inject ...
csm --namespace "user_42" probe ...
csm namespaces list
csm namespaces delete "user_42" --confirm
csm namespaces export "user_42" -o user_42.json
```

Default namespace `_default` preserves backward compatibility.

### MCP integration (ADR-0067)

MCP `initialize` accepts `clientInfo.namespace` → server pins namespace per session. Each MCP client is automatically isolated.

### Migration safety

- All existing rows tagged `_default` namespace on migration
- New deployments default to `_default`
- Tests verify pre-migration data still readable

## Pros and Cons

### Pros
- Single DB, single backup, single migration
- Cross-namespace queries possible if needed (admin/audit use cases)
- Backward compatible

### Cons
- Adds one TEXT column per row (~10 bytes overhead)
- Hot path adds one filter clause (~negligible with index)
- Admin must remember to scope CLI invocations

## Acceptance Criteria

- [ ] Migration `006_add_namespace.sql` applies cleanly to v0.3 DBs
- [ ] All Framework methods scoped by namespace
- [ ] CLI `--namespace` flag works on all subcommands
- [ ] `csm namespaces list/delete/export` work
- [ ] Existing tests pass with `_default` namespace
- [ ] New `tests/namespace_isolation.rs` proves isolation
- [ ] WASM API exposes `with_namespace`
- [ ] Per-file LOC ≤ 500
