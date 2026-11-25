# ADR-0105: Remote Backup and Restore Divergence — Export/Import Is the Path

## Status

Accepted (2026-11-25)

## Context

`Persistence::backup` and `Persistence::restore`
(`crates/csm-persistence/src/persistence_ops.rs`) implement snapshots with
libSQL mechanics that are local by construction:

- `backup` checkpoints, then runs `VACUUM INTO ?1` against the connection —
  a file the **engine's own host** must be able to create and write.
- `restore` ATTACHes a file path as `restore_db`, wipes and repopulates the
  tables inside one `BEGIN IMMEDIATE` transaction, and rebuilds namespace
  revisions — again through the locally-attached file.

A remote (Turso/libSQL URL) connection satisfies neither: the remote engine
has no path onto the operator's disk, and the networked protocol offers no
file ATTACH. Both operations refuse `local_path == None` with
`UnsupportedOperation`, which the 2026-11-25 audit (#1065) verified is the only
genuinely unsupported surface in the public persistence API — the wasm32 stub
and persistence-off refusals are by-design, not gaps.

Two designs were on the table for #1065:

1. **Implement remote backup/restore**: ns-scoped queries streaming a durable
   artifact to a local file, plus a restore path. This duplicates what the
   versioned export/import machinery (ADR-0100) already provides, while adding
   a second artifact format — its own schema migrations, its own validation
   surface — maintained for remote-only use.
2. **Keep the guard, make the refusal honest**: the error names the supported
   alternative, unit tests pin the contract, and the README documents the
   divergence where Turso users read.

## Decision

Option 2. The versioned export/import round-trip (`export` produces a
schema-tagged artifact, `import` validates then installs; replace by default,
merge opt-in) is the supported backup/restore path for remote databases.
`backup`/`restore` remain local-only. Second snapshot machinery is not built
while exactly one sufficient path exists.

## Consequences

- Both refusal sites name the export/import round-trip in the error message;
  prior messages named no alternative.
- Two lib-target unit tests pin the contract without a network endpoint: they
  construct the remote shape directly (`local_path: None` over a live local
  handle — the guards run before any I/O) and assert the `UnsupportedOperation`
  variant plus the constraint and workaround wording. The remote-shape
  construction also kills the `backup -> Ok(())` / `restore -> Ok(())`
  return-value mutants in the mutation profile.
- The README documents the divergence in a dedicated subsection adjacent to
  the Turso integration guide.
- A future demand export/import cannot serve (e.g. an artifact schema older
  than any shipped tool supports) reopens this decision via a new record; this
  ADR does not prohibit that, it stops the silent gap — the refusal is now
  honest, tested, and discoverable.
