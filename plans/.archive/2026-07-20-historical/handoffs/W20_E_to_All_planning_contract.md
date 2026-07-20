# Handoff: Group E -> All (Wave 20 Planning Contract)

## Scope Locked
- IQ-01..IQ-16 included; no discovered task dropped.
- Active sources only: `GOAP_STATE.md`, `GOAP_SKILL_MEMORY_HARDENING.md`, `swarm_audit_github_2026.md`.

## Sequencing Decisions
1. Run IQ-01/02/03 behind ADR + compatibility matrix first.
2. Execute IQ-05..IQ-10 as implementation batch with shared `.opencode/lib/skill-memory.sh` contracts.
3. Keep IQ-11/12 blocked until external npm Trusted Publisher access is available.

## Acceptance Contract
- Any breaking dependency upgrade requires ADR before merge.
- Every implementation task must emit tests or explicit test impact note to Group D.
- CI gate cannot close until Group D evidence is attached for all non-blocked tasks.
