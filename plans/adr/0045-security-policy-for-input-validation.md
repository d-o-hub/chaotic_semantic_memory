# ADR-0045: Security Policy for Input Validation

## Status

Proposed (backfilled 2026-05-01) - Wave 13

## Context

Security vulnerabilities in input handling:
- Bincode deserialization unlimited size (OOM DoS)
- Path traversal possible in file operations
- Metadata injection without validation
- Error messages may leak information

## Decision

Implement **security policy for input validation**.

**Proposed Measures:**
- Bincode deserialization size limits (100MB max)
- Path traversal protection for file operations
- Metadata size validation during concept building
- Error message sanitization (no token leakage)
- Association strength bounds checking

## Consequences

### Positive
- OOM attacks prevented
- Path traversal blocked
- Safe error messages
- Validated inputs throughout

### Negative
- Large imports rejected
- Path restrictions may limit valid use
- Error messages less detailed
- Validation overhead

## Implementation

- Module: src/framework_ops.rs, src/wasm.rs, src/singularity.rs
- Limits: bincode::DefaultOptions::new().with_limit(100MB)
- Validation: path canonicalization, bounds checking

## Sources

- ADR_REGISTRY.md: Security Policy for Input Validation
- ACTIONS.md lines 1861-1875 (add_bincode_size_limits action)
- Git: fix(framework): limit metadata filter recursion depth