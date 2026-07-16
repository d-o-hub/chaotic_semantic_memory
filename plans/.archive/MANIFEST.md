# Plan archive manifest

Generated: 2026-07-16T13:25:40Z

## Active (do not move without reference audit)

- `plans/GOAP_STATE.md` — canonical world state
- `plans/ACTIONS.md` — action queue
- `plans/GOALS.md` — goal targets
- `plans/ADR_REGISTRY.md` — ADR index
- `plans/adr/*.md` — architecture decisions
- `plans/WAVE_32_P2_PROGRESS.md` — current wave progress
- `plans/GOAP_AUDIT_2026_07_14.md` — wave 32 roadmap
- `plans/RECOMMENDATIONS_2026_07_14.md` — user-owned (never auto-archive)

## History candidates (immutable snapshots; safe to relocate with redirects)

- `plans/VERIFICATION_2026_04_29.md` → proposed `plans/.archive/history/VERIFICATION_2026_04_29.md`
- `plans/VERIFICATION_2026_04_30.md` → proposed `plans/.archive/history/VERIFICATION_2026_04_30.md`
- `plans/GAP_ANALYSIS_2026_04_30.md` → proposed `plans/.archive/history/GAP_ANALYSIS_2026_04_30.md`
- `plans/GAP_ANALYSIS_2026_06_26.md` → proposed `plans/.archive/history/GAP_ANALYSIS_2026_06_26.md`
- `plans/GOAP_ANALYSIS_2026_04_25.md` → proposed `plans/.archive/history/GOAP_ANALYSIS_2026_04_25.md`
- `plans/WAVE_21_P0_COMPLETION.md` → proposed `plans/.archive/history/WAVE_21_P0_COMPLETION.md`

## Redirects

When a file is moved, leave a stub at the old path:

```markdown
# Moved
This document was archived. See: plans/.archive/history/<name>
```

## Policy

1. Never bulk-delete; always manifest + redirect stubs.
2. Audit inbound links (`rg path plans AGENTS.md`).
3. User-owned recommendations require explicit approval.
