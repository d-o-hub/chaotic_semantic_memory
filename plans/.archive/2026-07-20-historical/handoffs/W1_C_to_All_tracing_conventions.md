# W1 C -> All: Tracing Conventions

Status: complete

Delivered conventions:
- Canonical field names:
  - concept ops: `concept_id`, `concept_count`
  - association ops: `from_id`, `to_id`, `strength`
  - batch/import/export ops: `batch_size`, `merge`, `path`
  - version/migration ops: `schema_version`, `from_version`, `to_version`
- Span boundaries:
  - Framework API methods own top-level spans (`inject_concept`, `probe`, `associate`, `load_replace`, `load_merge`, delete/backup/restore).
  - Persistence operations run as child spans from framework calls or test harness entrypoints.
  - Sequence processing spans include end-to-end timing and internal step loops remain uninstrumented to avoid trace noise.
- Error-context tags:
  - Include operation name and primary identifier (`concept_id` or association pair) in error messages.
  - Preserve source error chain when mapping storage/serialization failures.
  - Keep wasm and native behavior consistent: same semantic tags even when implementation paths differ.
