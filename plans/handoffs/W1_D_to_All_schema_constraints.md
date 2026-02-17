# W1 D -> All: Schema Constraints

Status: complete

Delivered constraints:
- Schema version contract:
  - `__schema_version` table is authoritative for persistence schema level.
  - Current baseline version is `1`; additive changes must increment via explicit migration step.
  - Runtime must initialize schema idempotently on startup.
- Migration ordering and rollback:
  - Apply migrations in ascending version order with transactional boundaries.
  - On failure, rollback current migration transaction and keep previous version marker unchanged.
  - Migration scripts must be re-runnable and safe against partially initialized databases.
- Compatibility requirements (export/import + version history):
  - `concept_versions` must preserve FK integrity (`ON DELETE CASCADE`) with `concepts`.
  - Export/import flows must maintain concept-first, association-second insertion ordering.
  - Import in replace mode must clear dependent tables before rehydrate to avoid orphaned associations.
