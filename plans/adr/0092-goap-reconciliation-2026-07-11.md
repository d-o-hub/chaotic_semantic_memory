# ADR-0092: GOAP Reconciliation 2026-07-11

## Status

Accepted

## Context

The GOAP world state (`plans/GOAP_STATE.md`) was last reconciled at commit
`7a0a432` (Wave 30, 2026-06-26). Since then, 13+ commits have landed on main
(HEAD now `87248dba`), including PRs #444, #463, #493, #495, #497, #498, #499,
#500, and #501. Key PRs #444 (BM25 sparse collection) and #94 (BM25 scoring
loop) are now merged but were tracked as `open` in GOAP state.

This ADR documents the reconciliation findings and corrective actions.

## Decision

Reconcile GOAP world state with the current codebase, documenting:

1. **LOC gate violations** (3 files in workspace crates now exceed 500 LOC)
2. **CI on main is failing** (commitlint scope violations from merged PRs)
3. **Supply chain advisory failures** (5 new Dependabot alerts not yet in deny.toml ignore list)
4. **Stale PR tracking** (PRs #444 and #94 merged but GOAP still shows them as open)
5. **Test count drift** (696 → 1029 test functions)
6. **Version drift** (0.3.6 → 0.3.7)
7. **HEAD drift** (7a0a432 → 87248dba, 13+ commits)

## Findings

### F1: LOC Gate Violations (3 workspace crate files)

| File | Lines | Over by |
|------|------:|--------:|
| `crates/csm-memory/src/singularity.rs` | 629 | +129 |
| `crates/csm-core-lib/src/hyperdim.rs` | 563 | +63 |
| `crates/csm-memory/src/graph_traversal.rs` | 517 | +17 |

**Root cause**: Workspace crate extraction (PRs #377-#385) moved code to
`crates/` but subsequent Jules bot PRs added code without LOC gate enforcement
on workspace crates. The CI LOC gate only checks `src/` via `find src -name '*.rs'`.

**Action**: Queue `fix_workspace_loc_gate` action (cost 5) to:
- Extend CI LOC gate to cover `crates/*/src/`
- Split the 3 violating files using the established pattern (extract submodule)

### F2: CI Commitlint Failures on Main

Two recent merged commits violate commitlint scope rules:
- `e8646c2 fix(cli-npm): handle EACCES gracefully in global install (#501)` — scope `cli-npm` not in allowed list
- `b649c7c Persist RetrievalAbstention events as absence-memory entries (#463)` — no conventional commit format at all

**Root cause**: Jules bot and contributor PRs merged without commitlint passing
(the `commitlint` job is not a required check for merge). The push-event
commitlint then retroactively fails when the non-compliant commits land on main.

**Action**: Queue `fix_commitlint_scopes` action (cost 2) to:
- Add `cli-npm` (or just validate under `cli`) to commitlint.config.cjs scope-enum
- Add ignore rule for Jules bot automated commits without scope
- OR: Make commitlint a required status check to prevent future violations

### F3: Supply Chain Advisory Failures

`cargo deny check advisories` is failing. 5 open Dependabot alerts:

| # | Severity | Package | Summary |
|---|----------|---------|---------|
| 22 | medium | opentelemetry_sdk | Unbounded memory allocation in W3C Baggage propagation |
| 21 | medium | time | Stack exhaustion DoS |
| 20 | low | lru | `IterMut` Stacked Borrows violation |
| 4 | low | libsql-sqlite3-parser | Crash on invalid UTF-8 |
| 1 | low | libsql-sqlite3-parser | Crash on invalid UTF-8 |

**Root cause**: New advisories published since last deny.toml update. The existing
`ignore` list only covers 3 older advisories (paste, bincode, number_prefix).

**Action**: Queue `update_deny_toml_advisories` action (cost 2) to triage and
either upgrade deps or add documented ignore entries for blocked-upstream alerts.

### F4: Stale PR Tracking

- PR #444 (BM25 sparse collection): GOAP shows `pr_444_status: open` — now MERGED
- PR #94 (BM25 scoring loop): GOAP shows `pr_94_status: open` — now MERGED

### F5: Test Count Drift

GOAP records `tests_count: 696` but actual count is **1,029** test functions.
Growth of 333 tests across Waves 29-30 and Jules bot PRs.

### F6: Open PR Status

Only PR #502 (SIMD Hamming distance, Jules bot) is open and MERGEABLE.

### F7: TODO Markers

One `TODO` remains at `src/retrieval/bm25.rs:109`:
```
/// TODO: Wire into the main hybrid retrieval pipeline for short-circuiting.
```

## Consequences

- GOAP_STATE.md updated with corrected HEAD, version, test count, and PR statuses
- ACTIONS.md updated with 3 new queued actions for Wave 31
- deny.toml advisory failures are a release blocker until addressed
- LOC gate violations must be fixed before next release (LOC gate is a documented hard constraint)
- CI commitlint failures on main are cosmetic (don't block other jobs) but should be fixed for hygiene

## Wave 31 Actions Queued

| Action | Cost | Priority |
|--------|------|----------|
| `fix_workspace_loc_gate` | 5 | P1 (hard constraint violation) |
| `fix_commitlint_scopes` | 2 | P2 (CI hygiene) |
| `update_deny_toml_advisories` | 2 | P1 (release blocker) |
| `merge_pr_502_simd_hamming` | 1 | P2 (perf improvement, already MERGEABLE) |
| `create_agents_context` | 3 | P3 (DX, carried from Wave 30) |
