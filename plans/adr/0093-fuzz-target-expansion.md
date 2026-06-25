# ADR-0093: Fuzz Target Expansion

## Status

Proposed

## Context and Problem Statement

The fuzz harness (`fuzz/fuzz_targets/`) contains 3 targets:
- `hvec_from_bytes` — HVec deserialization
- `reservoir_step` — Reservoir dynamics
- `persistence_save_concept` — Persistence write path

Several high-risk parsing surfaces lack fuzz coverage:
- JSON import/export parsing (untrusted data from files)
- Metadata filter deserialization (user-supplied filter expressions)
- BM25 tokenization/indexing (arbitrary text input)
- Bridge retrieval query parsing (complex query strings)

## Decision

Add 4 new fuzz targets:

1. **`fuzz_json_import`** — Feed arbitrary bytes to the JSON import parser.
   Exercises `BinaryExportPayload` deserialization and concept reconstruction.
2. **`fuzz_metadata_filter`** — Feed arbitrary JSON to `MetadataFilter`
   deserialization. Catches stack overflow from deeply nested And/Or/Not.
3. **`fuzz_bm25_tokenize`** — Feed arbitrary UTF-8 strings to BM25 tokenizer
   and indexing. Catches panics on edge-case Unicode, empty docs, huge docs.
4. **`fuzz_text_encoder`** — Feed arbitrary strings to `TextEncoder::encode`.
   Catches panics in FNV-1a hashing or positional encoding with extreme inputs.

Estimated cost: 4

## Consequences

- Increases fuzz surface from 3 to 7 targets.
- Catches panic/OOM bugs in parsing paths before they reach production.
- Integrates with existing `cargo +nightly fuzz` workflow.
- Each target is small (~20-30 lines) and self-contained.

## References

- `fuzz/fuzz_targets/` — Existing fuzz targets
- `src/export_payload.rs` — JSON import/export
- `src/metadata_filter.rs` — Filter deserialization
- `src/retrieval/bm25.rs` — BM25 tokenizer
- `src/encoder.rs` (crates/csm-core) — TextEncoder
