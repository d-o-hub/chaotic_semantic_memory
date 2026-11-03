# ADR-0103: GOAP Reconciliation and Plans Compaction 2026-11-03

## Status

Accepted

## Context

By 2026-11-03 the plans working set had drifted back to the shape ADR-0097
split apart: `plans/GOAP_STATE.md` was 638 lines / 72 KiB and `plans/ACTIONS.md`
55 KiB, most of it dated per-PR narrative appended below the active queue
("Last completed …" blocks for actions that closed months earlier). The
`plans/` root also carried 39 completed one-shot records — 36 `PR_ROAST_*`
files from 2026-09-04 through 2026-10-31, two superseded recommendation
backlogs, the persistence-owner dedupe analysis, and the test-surface audit —
that no current session needs but every `plans/` listing and search did.

The non-destructive archive policy is ADR-0096; the current-truth split is
ADR-0097. This decision re-applies both, because the earlier rule was stated
but not enforced by anything that notices re-growth.

## Decision

1. **Snapshot before rewrite.** Verbatim copies of both state files at
   `plans/.archive/2026-11-03-historical/{GOAP_STATE,ACTIONS}_2026_11_03.md`.
2. **Move completed dated records** into the same archive directory:
   `PR_ROAST_2026_09_*` and `PR_ROAST_2026_10_*` (keep 2026-11-01..03 active),
   `RECOMMENDATIONS_2026_07_20.md`, `RECOMMENDATIONS_2026_09_29.md`,
   `DEDUPE_PERSISTENCE_OWNER_BODIES_2026_08_13.md`, and
   `TEST_SURFACE_AUDIT_2026_09_18.md`.
3. **`GOAP_STATE.md` = current truth only**: every flag, metric and invariant
   kept, with one-line dated comments; the 123-key set is preserved exactly
   (plus `chaos_input_dim_rejected_not_clamped` from #961), and the full
   rationale for each key remains in the snapshot.
4. **`ACTIONS.md` = active queue only**: the six queued actions and the
   dependency reading, with no historical completion blocks.
5. **Update the archive ledger and pointers**: `plans/ARCHIVE_MANIFEST.md`
   gains a row for every archived file (bidirectional gate), and
   `plans/README.md` names the new archive root.
6. **Re-measure, don't carry forward**: `main_head`, `tests_count` (1188) and
   `dependabot_alerts_open` (3) are dated measurements from this session; the
   ADR counts move 98/97 -> 99/98 with this record.

## Consequences

- Session-start context load returns to roughly 250 lines of plan YAML.
- History is not deleted: the snapshot, the 39 moved records, git, and the
  manifest row per file keep every claim resolvable.
- The queue gate (`scripts/check-goap-queue-issues.sh`) and the archive gate
  (`scripts/check-archive-manifest.sh`) remain the sensors that notice drift,
  so the split does not depend on the next agent remembering ADR-0097.
- Risk: references to an archived filename in progress logs, skills or commit
  messages are no longer sibling paths. `plans/ARCHIVE_MANIFEST.md` is the
  redirect index; active docs updated in this change point at the archive path.

## References

- ADR-0096 (non-destructive archive policy)
- ADR-0097 (GOAP reconciliation and plans compaction, 2026-08-08)
- `plans/ARCHIVE_MANIFEST.md`, `plans/README.md`
