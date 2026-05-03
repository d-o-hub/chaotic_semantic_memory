# ADR-0043: Skill-Memory Security Hardening

## Status

Accepted (backfilled 2026-05-01) - Wave 12 Complete

## Context

Skill-memory library security vulnerabilities:
- No path traversal protection
- No input validation for skill names/concept IDs
- Silent error failures (stderr not captured)
- No audit logging
- Security score: 4/10 (before hardening)

## Decision

Implement **skill-memory security hardening**.

**Deliverables:**
- Path traversal protection (database must be in project directory)
- Input validation for skill names, concept IDs, metadata
- Error handling: captures CLI stderr, differentiated exit codes
- Structured JSON logging with severity levels
- Secure file permissions (700 owner-only)

## Consequences

### Positive
- Security score improved: 4/10 -> 8.5/10
- Path traversal blocked
- Invalid inputs rejected
- Detailed error messages
- Audit trail created

### Negative
- More complex shell library
- Validation overhead
- Breaking change for existing scripts
- Requires skill updates

## Implementation

- File: .opencode/lib/skill-memory.sh (v2.0.0)
- Exit codes: 0=success, 1=validation, 2=CLI, 3=jq
- Logging: CSM_LOG_LEVEL environment variable

## Sources

- ACTIONS.md: Skill-memory security actions
- W12b_SKILL_MEMORY_SECURITY_HARDENING.md handoff
- Git: security(cli): fix path hijacking commits