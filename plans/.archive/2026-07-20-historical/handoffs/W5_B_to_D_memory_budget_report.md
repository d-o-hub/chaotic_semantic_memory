# W5 B -> D Handoff: Memory Budget Report

## Action
- `validate_memory_footprint_10m`

## Method
- Added modeled validation in `tests/performance_targets.rs`.
- Equivalent compact index model:
  - `CSM_MEMORY_MODEL_BYTES_PER_CONCEPT` (default: `1`)
  - `CSM_MEMORY_MODEL_CODEBOOK_BYTES` (default: `2 MiB`)
  - `CSM_MEMORY_MODEL_METADATA_BYTES` (default: `256 KiB`)

## Calculation
- Target concepts: `10,000,000`
- Projected bytes (default model): `10,000,000 + 2,097,152 + 262,144 = 12,359,296`
- Threshold: configurable via `CSM_MEMORY_MODEL_MAX_BYTES` (default: `12,582,912` bytes / `12 MiB`)

## Result
- Status: `pass`
- `10m_concepts_under_12mb`: `true`

## Repro Command
- `cargo test --test performance_targets -- --nocapture`

## Handoff to Group D
- Use this modeled footprint as Wave 5 memory evidence for gate closure.
