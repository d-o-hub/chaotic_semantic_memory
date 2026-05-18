# Memory Lifecycle Checklist

## Save — 2026-05-18 ✅

- [x] concept IDs were injected successfully
  - `csm inject test::lifecycle::alpha` — OK
  - `csm inject test::lifecycle::beta` — OK
- [x] associations were created successfully
  - `csm associate alpha beta -s 0.9` — OK
- [x] concepts are discoverable through probe
  - `csm probe alpha` returns beta (similarity=0.006055) — OK

## Load — 2026-05-18 ✅

- [x] export file exists and is non-empty
  - `/tmp/opencode/memory-lifecycle/export.json` — 4128 bytes
  - checksum: `821d5a78240b2aadf69a00ae805d11b2c1579f00377481461306e0430639daf4`
- [x] import succeeds into a clean database
  - `csm import export.json --database roundtrip.db` — "imported 2 concepts"
- [x] loaded records preserve IDs and metadata
  - `csm get test::lifecycle::alpha` returns `{"phase":"save","v":1}` — OK

## Archive — 2026-05-18 ✅

- [x] archive marker or archive command output is recorded
  - Injected `archive::test::lifecycle::alpha` with metadata
- [x] archived target ID is referenced explicitly
  - Metadata: `{"target":"test::lifecycle::alpha","status":"archived"}`
- [x] archive timestamp exists
  - Metadata: `{"archived_at":1779121431}`

## Delete — 2026-05-18 ✅

- [x] deleted/tombstoned IDs are not returned as active
  - `csm probe alpha` — beta no longer in results
- [x] associations to deleted IDs are removed or blocked
  - No orphan associations remaining
- [x] no file/DB orphan mismatch remains
  - `csm stats` reports 0 concepts after full cleanup

## Cross-check — 2026-05-18 ✅

- [x] file artifacts and DB row counts are consistent
  - export.json: 2 concepts, 1 association
  - roundtrip.db `csm_concepts`: 2 rows, `csm_associations`: 1 row
- [x] rerunning verification is idempotent
  - DB lifecycle.db after full delete cycle shows 0 concepts
- [x] final evidence bundle attached
  - GOAP_STATE.md updated, ACTIONS.md updated, LEARNINGS.md updated
  - ADR-0083 documents export format contract
