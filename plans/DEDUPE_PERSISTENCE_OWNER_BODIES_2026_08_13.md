# Deduplicate Persistence Owner Bodies — Implementation Plan (2026-08-13)

Status: **Proposed — awaiting approval** (GOAP action `deduplicate_persistence_owner_bodies`, ADR-0094)

## Problem

ADR-0094 (2026-07-16) decided workspace crates own implementations and the root
`chaotic_semantic_memory` is a compatibility facade that may re-export/delegate/adapt
but may not retain a second algorithm implementation. Persistence still violates this:
the root crate implements persistence fully itself, and `csm-persistence` is a stale
near-duplicate second implementation with **zero code consumers**.

## Evidence (verified 2026-08-13, main @ 7b9d329)

### Root is the active implementation
- Root modules: `src/persistence.rs`, `persistence_ops.rs`, `persistence_concepts.rs`,
  `persistence_index.rs`, `persistence_migrations.rs`, `persistence_versions.rs`
  (all `#[cfg(all(not(wasm32), feature = "persistence"))]`), `persistence_wasm.rs`
  (wasm32, re-exported as `persistence`), plus root-only `bridge_persistence.rs`
  (canonical-concept/graph CRUD + AbsenceStore) and `index_envelope.rs`
  (ADR-0093 `IndexSnapshotEnvelope`).
- Every consumer uses the root facade `chaotic_semantic_memory::persistence::Persistence`:
  `framework.rs:16,31`, `framework_builder.rs:11,375-395`, `bridge_persistence.rs:8`,
  `persistence_{concepts,index,migrations,ops,versions}.rs`, 8 integration tests
  (`persistence_crud`, `persistence_ops_coverage`, `persistence_roundtrip`,
  `migration_errors`, `ann_revision_envelope`, `bridge_persistence_integration`,
  `turso_roundtrip`, `performance_targets`), `fuzz/fuzz_targets/persistence_save_concept.rs`,
  `benches/persistence_benchmark.rs`. CLI forwards `persistence` to root; `src/cli/`
  must not import `crate::persistence` directly (`tests/arch_fitness.rs`).

### csm-persistence crate is dead weight
- `csm_persistence` (import path) has ZERO matches in src/, crates/, tests/, benches/,
  examples/, wasm/, fuzz/, scripts/, .github/. All references are manifest declarations
  (root Cargo.toml:7,194,215,326), CI wiring, and docs.
- Crate `Persistence` is `#[allow(dead_code)]` (crates/csm-persistence/src/persistence.rs:15);
  only `pub use persistence::Persistence` is exported (lib.rs:21). Crate tests: 3 embedded
  libsql unit tests, run by CI "Test Workspace Crates (csm-persistence)".

### Divergence map (crate vs root)
- **Type identity is not the problem**: root `singularity.rs:2` = `pub use csm_memory::singularity::*`;
  `Concept<H: Hypervector = HVec10240>` (csm-memory), `HVec10240` from csm-core-lib. Same types both sides.
- **API**: crate method set is a strict subset of root. 29 common; **13 root-only**:
  3 absence (`get/upsert/list_absences`), 6 canonical/graph (`save/delete/load_canonical_concept`,
  `load_all_canonical_concepts`, `save/load_concept_graph`), 4 envelope/revision
  (`save/load_index_envelope`, `get/bump_namespace_revision`), + pub(crate)
  `record_concept_version(_scoped)`, `bump_namespace_revision_with_conn`, `load_all_associations`.
- **Schema ladder incompatible**: root v9=`csm_associations.created_at`, v10=`csm_absences`,
  v11=`csm_namespace_meta` (LATEST_SCHEMA_VERSION=11); crate v9=`csm_concepts.vector_format`,
  v10=`created_at` (LATEST_SCHEMA_VERSION=10). Root writes 8-col `csm_concepts`; crate writes
  9-col with `vector_format`. Cross-opened DBs miss tables/columns.
- **Behavior**: root writes `created_at` in `save_associations`/`restore`; crate does not.
- **Genericity**: crate uses `save_concept<H: Hypervector>`; root is concrete `Concept`
  (= `Concept<HVec10240>`). Signature-visible but type-compatible at the default.
- **Index**: crate `persistence_index.rs` (49L) only `save/load_index`; root (276L) adds
  envelope + namespace-revision + `load_all_associations` (ADR-0093).
- **Payloads in 3 forms**: root private `src/export_payload.rs` (pub(crate), concrete `Concept`);
  public parallel in `csm-traits` (`ExportPayload`/`ExportConcept`/`BinaryExportPayload`/
  `BinaryConcept`/`BinaryMetadataValue`, all pub); `csm-duckdb` permissive `ExportPayloadStub`
  (f64 associations — reads legacy JSON only).

### Pinned contracts that must survive (parity gates)
- Facade path `chaotic_semantic_memory::persistence::{Persistence, ConceptVersion}` (all tests).
- bincode byte layout of `BinaryExportPayload` (framework_namespaces tests :199-310 pin bytes).
- JSON wire: `import_json` deserializes into `ExportPayload{concepts: Vec<Concept>}`
  (framework_ops_import.rs:131) — legacy files must stay readable; duckdb stub unchanged.
- `schema_version` pinned (migration_errors.rs:49); v5 legacy-table migration pinned
  (persistence_roundtrip.rs:196).
- Local p50 < 20ms (performance_targets.rs:55, turso_roundtrip.rs:21); persisted-bytes metric.
- `src/cli/` never imports `crate::persistence` (arch_fitness).
- ADR-0094 no-feature stub (lib.rs:90-265) and `persistence_disabled.rs` behavior unchanged.

## Decision (recommended approach)

Per ADR-0094, promote the crate to canonical owner; root becomes facade/delegate. Root
behavior and the root schema ladder are canonical (production DBs + tests pin them); the
crate converges to them. Direction of delegation: root `persistence` module keeps its
public path but its body becomes re-exports/delegates of `csm_persistence`.

Phased, one PR per phase (each CI-green, mergeable independently):

### Phase 1 — Crate reaches root parity
PR: `refactor(persistence): converge csm-persistence to root schema + API (ADR-0094)`
- Crate migration ladder → root's exact ladder (v9 created_at, v10 csm_absences,
  v11 csm_namespace_meta); `LATEST_SCHEMA_VERSION = 11`; `created_at` writes in
  `save_associations`/`restore`.
- **Drop the crate's `vector_format` column** (verified 2026-08-13: crate
  `load_concept`/`load_all_concepts` never select it — persistence_concepts.rs:145-253
  read 6 columns; the type parameter `H` decides the format, so the column is
  write-only dead data). csm_concepts converges to root's 8-col schema; no extra
  migration needed and no impact on real (root-created) DBs.
- Port root-only surface into crate: envelope/revision (`save/load_index_envelope`,
  `get/bump_namespace_revision`, `load_all_associations`), `AbsenceStore`
  (`get/upsert/list_absences`), `record_concept_version`.
- Move `IndexSnapshotEnvelope` (`src/index_envelope.rs`, ADR-0093) to `csm-memory`
  (owns ANN indexes per ADR-0094); root re-exports, crate imports.
- Crate parity tests: mirror `ann_revision_envelope`, `bridge_persistence_integration`,
  `migration_errors`, `persistence_roundtrip`, absence-store behaviors at crate API level.
- Watch 500-LOC caps: split crate `persistence.rs` (AbsenceStore → `persistence_absence.rs`).

### Phase 2 — Root delegates to crate
PR: `refactor(persistence): root persistence facade delegates to csm-persistence (ADR-0094)`
- `src/persistence.rs` keeps `pub mod persistence` path; body → `pub use
  csm_persistence::{Persistence, ConceptVersion}` + thin adapters. `persistence_ops.rs`
  / `_concepts.rs` / `_index.rs` / `_migrations.rs` / `_versions.rs` impls move to crate;
  root modules become re-export shells or are deleted with their lib.rs decls.
- wasm32: crate `persistence_wasm` becomes canonical; root re-exports it as `persistence`;
  root's wasm32 dep `features = ["wasm"]` (Cargo.toml:215) becomes live, no longer vestigial.
- No-feature stub, arch_fitness constraint, CLI layering: unchanged.
- **Fork B (verified): canonical/graph/absence CRUD moves into the crate.** Phase 1 puts
  the migration ladder (incl. `csm_absences` v10, `csm_namespace_meta` v11) in the crate,
  making those tables crate-owned durable schema — root CRUD over them would be a second
  owner (ADR-0094 violation). `CanonicalConcept`/`ConceptGraph`/`AbsenceEntry` (exist only
  in root today; grep over crates/ → no matches) are promoted to `csm-traits` with verbatim
  serde derives; root re-exports them, so `chaotic_semantic_memory::semantic_bridge::*`
  and `bridge_persistence::*` paths stay stable. The crate implements the CRUD over the
  promoted types; `src/bridge_persistence.rs` becomes a delegate then is deleted.

### Phase 3 — Payload convergence
PR: `refactor(export): root export payloads delegate to csm-traits (ADR-0094)`
- `src/export_payload.rs` becomes an adapter over `csm_traits::{ExportPayload,
  ExportConcept, BinaryExportPayload, BinaryConcept, BinaryMetadataValue}` with
  `Concept`↔`ExportConcept` conversion; delete root parallel types after adoption.
- bincode bytes stay identical (pinned tests); JSON import keeps reading legacy files;
  duckdb stub untouched.

### Phase 4 — Cleanup and full verification
- Remove `#[allow(dead_code)]` from crate; delete root duplicate bodies; LOC gate re-check
  (root files shrink, crate files stay ≤500).
- Full gates: `./scripts/validate.sh`, `cargo test --test cli_parity --features cli`,
  `cargo test --test arch_fitness`, mutation, no-default-features tree lean, check-adr-parity.

## Decision forks — verified recommendations (analysis-swarm 2026-08-13)

Analysis: three-persona swarm (RYAN methodical / FLASH shipping / SOCRATES questioning)
executed inline over scout evidence + web research (persona subagents were provider
rate-limited; all evidence was already gathered). External research: vector databases
treat vector format as collection/schema-level configuration, not a per-row discriminator
(Weaviate named vectors, Qdrant multi-vector named fields, pgvector typed columns, Milvus
schema); owner-neutral storage-trait workspace crates are an established pattern
(ferrant `Storage`, servo `storage-traits`, meta-language `LinkStore`).

- **Fork A — keep generic `<H: Hypervector>` API, DROP `vector_format` column.**
  Verified in crate source: `load_concept`/`load_all_concepts` (persistence_concepts.rs
  :145-253) never SELECT `vector_format` — the caller's type parameter decides the format,
  so the column is write-only dead data and the schema divergence it causes is removable
  with zero behavior loss. Keeps ADR-0094's owner-neutral intent (generic API, monomorphized
  at H=HVec10240 = zero runtime cost, type-identical to root's concrete facade) while
  converging the schema exactly to root's 8-col ladder. No speculative column, no migration
  surface on real DBs. A second format, if ever needed, is a schema-level migration — the
  industry-standard approach.
- **Fork B — move canonical/graph/absence CRUD into the crate.** Once Phase 1 moves the
  migration ladder into the crate, `csm_canonical`/`csm_absences` are crate-owned durable
  schema; root CRUD over them would be a second implementation owner, which is the exact
  ADR-0094 violation this action exists to remove. Promotion of the three types to
  `csm-traits` (verified absent from all crates today) with verbatim serde derives and root
  re-exports keeps every public path and wire format stable; `bridge_persistence_integration`
  tests continue to pin behavior through the facade. Cost is contained inside Phase 2.

Consensus trade-offs acknowledged:
- Generic API + schema convergence keeps a small untested-generics surface (mitigated by
  H=HVec10240 parity tests; there is no second Hypervector type to test against today).
- Type promotion touches `src/semantic_bridge.rs` but only moves type definitions; root
  orchestration stays put (ADR-0094:49 root-specific adapters/orchestration).
- Validation: crate parity tests green in Phase 1; `grep -rn "impl Persistence" src/` empty
  and all 8 facade integration tests green after Phase 2.

## Risks

- Schema migration on real DBs: only root-created DBs exist in practice (crate has zero
  consumers) → ladder convergence is one-directional; still verify v11 → open → v11.
- bincode/JSON wire stability is the highest-risk area (pinned bytes, legacy files).
- Type promotion (Fork B): serde derives move verbatim to csm-traits; roundtrip/wire
  tests (bridge_persistence_integration, export/import) re-verified after promotion.
- Generic-vs-concrete facade: at `H = HVec10240` types are identical; callers using bare
  `Concept` are unaffected. Signature snapshots added in Phase 1 parity tests.
- Perf: delegation adds an in-process call layer; p50<20ms targets re-verified.
- LOC caps on crate files as surface grows (mitigated by module splits).

## Acceptance (definition of done)

- `cargo test -p csm-persistence --features persistence` green incl. new parity tests.
- All 8 root persistence integration tests green via the facade after delegation.
- `grep csm_persistence` in root `src/` now has matches only in the facade/delegate modules;
  root `src/persistence*.rs` contains no algorithm bodies.
- `chaotic_semantic_memory::persistence` public path unchanged; wasm32 re-export works.
- validate.sh, cli_parity, arch_fitness, check-adr-parity, LOC gate, no-default lean, perf
  p50<20ms all green; CI green on each phase PR.
- GOAP: `deduplicate_persistence_owner_bodies` marked complete; ADR-0098-style reconciliation
  note in ACTIONS.md; GOAP_STATE `action_last_completed` updated.

## Files touched (inventory)

- Crate: `crates/csm-persistence/src/{lib,persistence,persistence_ops,persistence_concepts,
  persistence_index,persistence_migrations,persistence_versions,persistence_wasm}.rs`,
  `+ persistence_absence.rs`, `Cargo.toml`.
- Root: `src/{persistence,persistence_ops,persistence_concepts,persistence_index,
  persistence_migrations,persistence_versions,persistence_wasm,bridge_persistence,
  index_envelope,export_payload,framework_persistence}.rs`, `src/lib.rs`, `Cargo.toml`.
- Shared: `crates/csm-memory` (IndexSnapshotEnvelope), `crates/csm-traits`
  (payload types already present; possibly CanonicalConcept/ConceptGraph/AbsenceEntry).
- Tests: `tests/*` (unchanged — they are the parity gates), `crates/csm-persistence/src/**`
  embedded tests.
