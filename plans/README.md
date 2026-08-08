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
| [`ARCHIVE_MANIFEST.md`](ARCHIVE_MANIFEST.md) | What moved where (2026-07-20 + 2026-08-08 compactions) |
| [`.archive/2026-07-20-historical/`](.archive/2026-07-20-historical/) | Immutable historical plans + handoffs |
| [`.archive/2026-08-08-historical/`](.archive/2026-08-08-historical/) | Pre-compaction GOAP_STATE / ACTIONS snapshots (ADR-0097) |

## Hygiene rules

- Keep `action_last_completed` **exactly once** in `GOAP_STATE.md` (last key; YAML last-key-wins).
- Action statuses: `queued` | `in_progress` | `complete` | `blocked` | `deferred`.
- **`ACTIONS.md` is the active queue only** (ADR-0097): remove actions when they
  complete; full history lives in `plans/.archive/2026-08-08-historical/` + git.
- **`GOAP_STATE.md` is current truth only** (ADR-0097): update flags in place
  with a short dated comment; dated per-wave/PR narrative belongs in
  `plans/.archive/` snapshots, not appended here.
- Prefer archiving completed one-shots over deleting.
- New recommendations: dated file `RECOMMENDATIONS_YYYY_MM_DD.md` + queue ACTIONS entries.
