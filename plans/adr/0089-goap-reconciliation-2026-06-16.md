# ADR-0089: GOAP Reconciliation 2026-06-16

## Status

Accepted

## Context and Problem Statement

A codebase audit on 2026-06-16 (post PRs #389, #394, #396) found two categories
of state drift in the GOAP planning files:

1. **Duplicate `action_last_completed` key** in `plans/GOAP_STATE.md`. The file
   contained two `action_last_completed:` lines (lines 1366 and 1398, set to
   `csm_wasm_created_2026_06_12` and `goap_orchestration_2026_06_13`
   respectively). YAML's last-key-wins rule silently makes the first one dead.
   This violates the project's own LEARNINGS.md invariant: "Always
   `grep -c '^  action_last_completed' plans/GOAP_STATE.md` -> must equal 1."
   The same footgun was previously fixed in ADR-0085 (2026-06-06).

2. **Stale "deferred" entries**. Three actions in `plans/ACTIONS.md` and the
   `goap_2026_06_13_deferred_actions` list in `plans/GOAP_STATE.md` were marked
   `deferred` despite their underlying work having already shipped:

   - **`add_otlp_grpc_exporter`** — completed by PR #396 (commit `1cacc8e0`,
     merged 2026-06-14). Added `src/observability/otlp_grpc.rs` (111 LOC),
     wired `otlp_endpoint` into `ObservabilityConfig::init()`, and gated the
     feature behind `cfg(not(target_arch = "wasm32"))`.
   - **`deferred_performance_phase2`** — all three sub-components shipped:
     SIMD (`crates/csm-core/src/hyperdim_simd.rs`, `bundle_simd.rs`,
     `hyperdim_simd_bundle.rs`), LSH index (`crates/csm-memory/src/index/lsh.rs`),
     and Product Quantization via PR #389 (ADR-0075, merged 2026-06-14).
   - **`deferred_namespace_isolation`** — `src/framework_namespaces.rs`
     (14055 bytes) provides namespace APIs with CWE-770 input validation
     (PR #348/#349). ADR-0084 (2026-05-20) already called this out as
     implemented; the ACTIONS.md entry was never updated to reflect it.

Additionally, the `goap_2026_06_13_gap_analysis_status` block listed
`f10_quantized_hvs: in_progress` and a `# gRPC deferred (#391)` comment on
`f6_observability` — both stale for the same reason.

## Decision Drivers

- Restore the LEARNINGS.md DRY invariant: exactly one `action_last_completed`
  key in `GOAP_STATE.md`. Duplicate keys silently overwrite and mislead every
  downstream agent that reads the file.
- Align planning records with `main` so future sessions don't re-attempt
  already-shipped work. Three of four "deferred" items were already done.
- Maintain ADR registry <-> disk parity and pass `scripts/check-adr-parity.sh`
  and `scripts/validate.sh`.
- Match the established reconciliation pattern (ADR-0084 on 2026-05-20,
  ADR-0085 on 2026-06-06) so the audit trail remains readable.

## Considered Options

### Option 1: Reconcile drift, mark stale deferred actions complete, remove the duplicate key

- Good: planning files match codebase reality.
- Good: restores the single-key DRY invariant.
- Good: narrows the genuinely-deferred backlog to two items
  (`advanced_ttl_policies`, `association_decay`) — both correctly deferred
  per their ADR activation triggers.

### Option 2: Leave GOAP as-is

- Bad: drift cascades. The next planning agent that reads
  `goap_2026_06_13_deferred_actions` will queue `otlp_grpc_exporter` and
  `performance_phase2` as work to do, then re-discover they're already
  shipped.
- Bad: violates the AGENTS.md / LEARNINGS.md hard constraint on the
  `action_last_completed` key.

## Decision Outcome

Chosen option: **Option 1 - Reconcile drift, mark stale deferred actions
complete, remove the duplicate key**.

### Positive Consequences

- `action_last_completed` appears exactly once
  (`goap_reconciliation_2026_06_16`).
- The genuine deferred backlog is now precisely two items
  (`advanced_ttl_policies`, `association_decay`), both correctly deferred
  per their ADR activation triggers (no user demand yet).
- ADR registry remains in sync with disk.

### Negative Consequences

- None identified.

## Follow-up Actions

- [x] Remove the duplicate `action_last_completed: csm_wasm_created_2026_06_12`
      from `plans/GOAP_STATE.md` (converted to a comment for history).
- [x] Set the single `action_last_completed: goap_reconciliation_2026_06_16`.
- [x] Update `goap_2026_06_13_deferred_actions` to remove the 3 stale entries.
- [x] Update `goap_2026_06_13_gap_analysis_status.f10_quantized_hvs` from
      `in_progress` to `complete` (PR #389).
- [x] Update `goap_2026_06_13_gap_analysis_status.f6_observability` comment
      (gRPC no longer deferred).
- [x] Mark `add_otlp_grpc_exporter` action `complete` in `plans/ACTIONS.md`.
- [x] Mark `deferred_performance_phase2` action `complete` in `plans/ACTIONS.md`.
- [x] Mark `deferred_namespace_isolation` action `complete` in `plans/ACTIONS.md`.
- [x] Register ADR-0089 in `plans/ADR_REGISTRY.md`.
- [ ] Run `scripts/check-adr-parity.sh` to confirm registry <-> disk parity
      (CI will verify).

## References

- ADR-0084: prior GOAP reconciliation (2026-05-20)
- ADR-0085: prior GOAP reconciliation (2026-06-06, fixed same duplicate-key issue)
- PR #389: Quantized Binary Hypervectors (ADR-0075)
- PR #396: OTLP gRPC exporter + property-based security tests + csm-cli fix
  + error remediation hints (issues #391, #392, #393, #395)
- LEARNINGS.md: "State Drift Verification (Wave 21 P0 - May 2026)" section
