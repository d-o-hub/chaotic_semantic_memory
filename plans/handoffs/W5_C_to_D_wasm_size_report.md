# W5 C -> D Handoff: WASM Size Report

## Action
- `validate_wasm_binary_size`

## Required Inputs
- Release build command with fixed features
- Artifact path used for byte-size measurement
- Size measurement command and units

## Output Contract
- Artifact size in bytes and KB
- Explicit pass/fail against target: `wasm_binary_under_500kb`
- Reproducible command snippet for CI/local verification

## Consumption by Group D
- Attach as mandatory evidence in Wave 5 final gate decision
