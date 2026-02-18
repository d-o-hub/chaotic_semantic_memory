# W5 C -> D Handoff: WASM Size Report

## Action
- `validate_wasm_binary_size`

## Measurement
- Command: `cargo build --target wasm32-unknown-unknown --release --features wasm`
- Artifact: `target/wasm32-unknown-unknown/release/csm.wasm`
- Size: `5186` bytes (`5.06` KiB)
- Threshold: `512000` bytes (configurable via `CSM_WASM_SIZE_MAX_BYTES`)

## Result
- Status: `pass`
- `wasm_binary_under_500kb`: `true`
