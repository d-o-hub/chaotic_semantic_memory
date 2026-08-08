# ADR-0097: GOAP Reconciliation and Plans Compaction 2026-08-08

## Status

Accepted

## Context

`plans/GOAP_STATE.md` had grown to 1,892 lines and `plans/ACTIONS.md` to
5,106 lines. Both mixed *current world state* with five months of per-wave and
per-PR historical narrative (2026-02 → 2026-08). The 2026-07-20 compaction
(ADR-0096 era) archived one-shot plan documents but left the two state files
append-only, so session-start context load (AGENTS.md Phase 1) kept getting
more expensive and stale entries accumulated:

- **F1 — Dead duplicate key**: `benchmark_workspace_tests_run_in_ci` appeared
  twice (`false` at line 260, `true` at line 1721). YAML last-key-wins made
  the first entry silently dead — the exact failure mode ADR-0089 fixed for
  `action_last_completed`.
- **F2 — Stale statuses**: `framework_ops_perf_524_525_526` was recorded as
  `status: open_pr` although PR #532 had merged; `wave_33_queued` listed
  actions that `ACTIONS.md` already marked `complete`.
- **F3 — Two `in_progress` actions actually complete**:
  `harden_public_f32_api_validation` (PR #607 merged 2026-08-07; validators
  verified in `crates/csm-memory/src/singularity_decay.rs` and
  `src/wasm_ext.rs`) and `recover_v037_failed_deployments` (recover mode in
  `release.yml`, wasm-opt `--enable-nontrapping-float-to-int` enabled, and
  v0.3.7 + v0.3.8 both live on crates.io).
- **F4 — Malformed YAML**: the `recent_changes` block mixed 3/4/6-space
  indentation (unparseable list structure).
- **F5 — Queue unreadable**: 292 completed actions vs 8 active ones; the
  active queue started at line 4,592 of 5,106.

External practice check (2026-08-08): adr.github.io and MADR 4.0.0 (current
release, 2024-09-17) both frame the decision log as *immutable records* plus a
*current-state index* — records never shrink, but the working set stays small.
This repo already follows that split for ADRs (`plans/adr/` + registry +
parity gate); this ADR extends the same principle to GOAP state files.

## Decision

Split *current state* from *history*, non-destructively (ADR-0096 policy:
archive, never delete):

1. **Snapshot before rewrite.** Verbatim copies of both files at
   `plans/.archive/2026-08-08-historical/{GOAP_STATE,ACTIONS}_2026_08_08.md`.
2. **`GOAP_STATE.md` = canonical current truth only** (~120 lines): core
   flags, current metrics with date comments, plans pointers, active wave,
   flags that are currently `false` (the real backlog), landed invariants that
   constrain future work, and a single trailing `action_last_completed`.
   All dated per-wave/PR narrative lives in the archive snapshot.
3. **`ACTIONS.md` = active queue only** (`queued`/`in_progress`/`blocked`/
   `deferred`). Completed entries are not re-added; the archive snapshot +
   git history are the record. 6 actions remain queued.
4. **Corrections applied**: F1 duplicate removed; F2 stale statuses dropped
   with the narrative (truth now lives in the single active queue); F3 both
   actions recorded complete; F4 eliminated by the rewrite.
5. **Hygiene rules codified** in `plans/README.md` and both file headers:
   `action_last_completed` exactly once; update flags in place instead of
   appending blocks; dated snapshots go to `plans/.archive/`.

## Consequences

- Session-start load (AGENTS.md Phase 1) drops from ~7,000 to ~250 lines of
  plans YAML.
- Single source of truth per concern: queue in `ACTIONS.md`, state flags in
  `GOAP_STATE.md`, history in `plans/.archive/` + git.
- `scripts/plans-manager.sh` stays CRUD tooling; its lossy `archive
  completed` (names-only via grep) is **not** the mechanism for this split —
  dated verbatim snapshots are. A latent `cmd_clean` syntax bug (`AD #
  Remove duplicateRs`) was fixed in the same pass.
- Risk: agents citing old nested keys (e.g. `wave_33_queued`,
  `goap_2026_07_11_*`) must resolve them via the archive snapshot. Mitigated
  by the pointer headers at the top of both compacted files.

## References

- MADR 4.0.0 — https://adr.github.io/madr/ (decision log = immutable records + index)
- ADR organization guidance — https://adr.github.io/
- ADR-0096 (agent skill and workflow validation; non-destructive archive policy)
- ADR-0084 / 0085 / 0089 / 0092 (prior GOAP reconciliations)
