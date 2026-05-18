# GOAP: Memory Lifecycle Verification Follow-up

## Current State (2026-05-18)

The `memory-lifecycle-verification` skill was dogfooded against the
codebase. It revealed 4 concrete gaps in the skill's reference files
and in `plans/GOAP_STATE.md`:

### Gap 1: `sql_checks.sql` — Wrong Table Names
- Table `concepts` → actual name is `csm_concepts`
- Table `associations` → actual name is `csm_associations`
- Column `source_id` → actual column is `from_id`
- Column `target_id` → actual column is `to_id`
- **Risk**: Anyone running these reference queries gets "no such table"

### Gap 2: VALIDATION_CHECKLIST.md — Unmarked Checkboxes
- All checkboxes are `[ ]` — never been filled for any run
- No way to trace which verification run covered which items

### Gap 3: GOAP_STATE.md — Stale Flags
- `verification_2026_04_30_archive_phase_skipped: true` — reason
  was "no native CLI command" but archive markers are now a working pattern.
  Flag is misleading: the archive phase IS possible, just not via a
  dedicated subcommand.
- `verification_2026_04_30_delete_phase_skipped: true` — same reason,
  but `csm delete` command exists and works. Flag is outright wrong.

### Gap 4: Export JSON Associations Format (ADR-0083)
- `csm export` serializes associations as flat arrays:
  `["from_id", "to_id", 0.9]`
- This is fragile: no field names, position-dependent
- Alternative: `{"from_id": "...", "to_id": "...", "strength": 0.9}`
- Decision needed on whether to break the format or document the
  current contract

## Target State

```yaml
sql_checks_table_names_fixed: true
validation_checklist_2026_05_18_marked: true
goap_stale_archive_flag_fixed: true
goap_stale_delete_flag_fixed: true
adr_0083_export_format_decided: true
```

## Action Plan

| # | Action | Cost | Preconditions | Effect |
|---|--------|------|---------------|--------|
| 1 | Fix `sql_checks.sql` table/col names | 1 | Gap 1 identified | `sql_checks_table_names_fixed` |
| 2 | Mark VALIDATION_CHECKLIST.md for 2026-05-18 | 1 | Gap 2 identified | `validation_checklist_2026_05_18_marked` |
| 3 | Fix GOAP_STATE stale archive flag | 1 | Gap 3 identified | `goap_stale_archive_flag_fixed` |
| 4 | Fix GOAP_STATE stale delete flag | 1 | Gap 3 identified | `goap_stale_delete_flag_fixed` |
| 5 | Write ADR-0083 for export format | 2 | Gap 4 identified | `adr_0083_export_format_decided` |
| 6 | Update ACTIONS.md with actions 1-5 | 1 | 1-5 complete | `actions_md_lifecycle_followup_synced` |

Total cost: 7
