# Handoff: Group A -> Group D (Wave 20 Implementation)

## Tasks
- IQ-05 retry logic
- IQ-06 health checks
- IQ-07 metrics collection
- IQ-08 log rotation
- IQ-09 optional encryption
- IQ-10 rate limiting

## Assumptions Passed
- Skill-memory shell API remains centralized in `.opencode/lib/skill-memory.sh`.
- Retry backoff defaults: 1s/2s/4s, max 3 attempts.
- Log rotation defaults: 10MB, 5 retained files.

## Required Tests for D
- Lock-contention retry integration tests
- Corruption/health-check detection tests
- Metrics emission schema and percentile calculations
- Rotation atomicity and retention edge cases
- Encryption opt-in/opt-out behavior tests
- Per-skill rate-limit window and exhaustion tests
