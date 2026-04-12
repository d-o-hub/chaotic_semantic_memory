# Handoff: Group B -> Group D (Wave 20 Fix/Security)

## Tasks
- IQ-13 add `#[source]` chains for relevant error variants
- IQ-14 add remediation-oriented error context hints

## Assumptions Passed
- Public error enum stability is preserved.
- Added context must be deterministic and testable (no nondeterministic text).

## Required Tests for D
- Source-chain propagation tests (`source()` present where expected)
- Error message/context snapshot tests for key failure paths
