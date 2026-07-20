# Plans Index

Operational planning for `chaotic_semantic_memory`. Load these **before** implementing.

## Start here (every session)

1. [`GOAP_STATE.md`](GOAP_STATE.md) — world state, flags, last completed action  
2. [`ACTIONS.md`](ACTIONS.md) — queued / complete actions  
3. [`GOALS.md`](GOALS.md) — target goals  
4. [`RECOMMENDATIONS_2026_07_20.md`](RECOMMENDATIONS_2026_07_20.md) — current improvement backlog  

## Active roadmaps

| Doc | Purpose |
|-----|---------|
| [`GOAP_AUDIT_2026_07_14.md`](GOAP_AUDIT_2026_07_14.md) | Wave 32: correctness, ownership, evidence, agent safety |
| [`GOAP_ORCHESTRATOR.md`](GOAP_ORCHESTRATOR.md) | PR triage / orchestrator workflow |
| [`RECOMMENDATIONS_2026_07_20.md`](RECOMMENDATIONS_2026_07_20.md) | Full analysis: missing work, perf, docs, skills, features |

## Architecture decisions

| Doc | Purpose |
|-----|---------|
| [`ADR_REGISTRY.md`](ADR_REGISTRY.md) | Registry ↔ disk parity |
| [`adr/`](adr/) | Individual ADRs (0093–0096 Wave 32) |

## Archive

| Doc | Purpose |
|-----|---------|
| [`ARCHIVE_MANIFEST.md`](ARCHIVE_MANIFEST.md) | What moved where (2026-07-20 compaction) |
| [`.archive/2026-07-20-historical/`](.archive/2026-07-20-historical/) | Immutable historical plans + handoffs |

## Hygiene rules

- Keep `action_last_completed` **exactly once** in `GOAP_STATE.md`.
- Action statuses: `queued` | `in_progress` | `complete` | `blocked` | `deferred`.
- Prefer archiving completed one-shots over deleting.
- New recommendations: dated file `RECOMMENDATIONS_YYYY_MM_DD.md` + queue ACTIONS entries.
