# W1 A -> B: Fuzz Findings

Status: complete

Summary:
- Added `cargo-fuzz` harness with targets:
  - `hvec_from_bytes`
  - `reservoir_step`
  - `persistence_save_concept`
- Targets compile against current crate APIs and exercise malformed/edge inputs.

Baseline findings:
- No crash repro captured yet (compile-and-wire pass only, no long fuzz campaign run).
- `HVec10240::from_bytes` path now has dedicated malformed-byte harness coverage.
- `reservoir_step` target intentionally varies input length to stress mismatch handling.

SIMD safety notes for Group B:
- Keep byte parsing and bounds checks in scalar-safe path before SIMD lane operations.
- Do not assume aligned 1280-byte input in preprocessing; malformed lengths are common.
