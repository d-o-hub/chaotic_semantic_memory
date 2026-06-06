# ADR-0085: GOAP Reconciliation 2026-06

## Status

Accepted

## Context and Problem Statement

A codebase audit on 2026-06-06 found state drift between `main` and the GOAP
planning records in `plans/` (`GOAP_STATE.md` and `ACTIONS.md`). Several merged
pull requests had landed in `main` without corresponding world-state flags or
action entries, and one bad merge had reintroduced a duplicate key.

Unrecorded merged work:

1. **#345 — `perf(encoder)`**: reduced redundant allocations in the text
   encoding hot path (`src/encoder.rs`).
2. **#348 — `fix(framework)`** and **#349**: namespace input validation. Added
   `validate_namespace()` to `src/framework_validation.rs` (length ≤ 128 bytes,
   non-empty, no control characters), changed `set_namespace`/`with_namespace`
   to return `Result`, and applied the guard across the set/delete/export
   namespace APIs. Security rationale: namespace is used as a DB primary-key
   prefix and hash-map key; unbounded or control-character input could cause
   resource exhaustion (CWE-770) or corrupt row lookups.
3. **#351 — `ci`**: restricted the Miri job to `push` events (effectively main)
   to reduce CI cost on pull requests.

Additionally, PR #348 re-introduced a stale `action_last_completed:
pin_github_actions_to_sha` line into `GOAP_STATE.md` as a merge artifact,
producing two `action_last_completed` keys. Per AGENTS.md, this key MUST appear
exactly once (YAML last-key-wins makes earlier duplicates silently dead).

## Decision Drivers

- Keep GOAP planning files an accurate canonical source of truth so future
  sessions build on real capabilities and avoid redundant work.
- Enforce the AGENTS.md DRY invariant: a single `action_last_completed` key.
- Maintain ADR registry ↔ disk parity and pass all validation gates.

## Considered Options

### Option 1: Reconcile drift, record merged work, remove the duplicate key

Add world-state flags and an action entry for #345/#348/#349/#351, remove the
duplicate `action_last_completed` merge artifact, and document the change in this
ADR.

- Good, because planning files match codebase reality.
- Good, because it restores the single-key DRY invariant.
- Good, because it captures the namespace security hardening as durable record.

### Option 2: Leave GOAP as-is

- Bad, because drift cascades and misleads future planning agents.
- Bad, because the duplicate key violates the AGENTS.md hard constraint.

## Decision Outcome

Chosen option: **Option 1 — Reconcile drift, record merged work, remove the
duplicate key**.

The planning files are the canonical source of truth for agent-based execution;
aligning them with `main` preserves the integrity of the autonomous development
workflow.

### Positive Consequences

- World state now reflects encoder allocation reduction, namespace input
  validation (CWE-770 hardening), fallible namespace APIs, and Miri CI scoping.
- `action_last_completed` appears exactly once (`goap_reconciliation_2026_06`).
- ADR registry remains in sync with disk.

### Negative Consequences

- None identified.

## Follow-up Actions

- [x] Remove the duplicate `action_last_completed: pin_github_actions_to_sha`
      from `plans/GOAP_STATE.md`.
- [x] Add reconciliation world-state flags and set the single
      `action_last_completed: goap_reconciliation_2026_06`.
- [x] Add `goap_reconciliation_2026_06` action to `plans/ACTIONS.md`.
- [x] Register ADR-0085 in `plans/ADR_REGISTRY.md`.
- [ ] Run `scripts/check-adr-parity.sh` to confirm registry ↔ disk parity.
