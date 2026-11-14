# ACTIONS — Active GOAP Action Queue

> **Compacted 2026-11-03 (ADR-0097 pattern).** This file holds **active
> actions only** (`queued` | `in_progress` | `blocked` | `deferred`); completed
> entries are not re-added. A verbatim pre-compaction snapshot — including the
> long per-action "last completed" narratives — is archived at
> `plans/.archive/2026-11-03-historical/ACTIONS_2026_11_03.md`. Earlier history:
> `plans/.archive/2026-08-08-historical/`, `plans/.archive/2026-07-20-historical/`
> and git.
>
> More recent completions are recorded in `plans/GOAP_STATE.md`
> (`action_last_completed`, exactly once, last key) and in git.
>
> Hygiene: when an action completes, remove it here, set
> `action_last_completed` in `GOAP_STATE.md`, and let the next dated
> reconciliation snapshot the file. Do not re-add completed entries.

```yaml
# Queue empty as of 2026-11-08 (roast_and_close_zero_impact_prs_1016_1017_1018).
# The #1015 keeper, publish_crates_io_over_oidc_from_dispatch (ADR-0104), landed as
# 53052df1 immediately before this triage and is recorded in GOAP_STATE.md.
# The triage closed #1018/#1017/#1016 as zero-impact; record:
# plans/PR_ROAST_2026_11_08.md. Neither entry is re-added here.
# The earlier action, bound_chaotic_lsh_and_voronoi_lsh_input_dim (#961), landed as
# PR #1008 -> 546727d0.
# Queue empty in this branch as of 2026-11-12: Issue #1002's validator and
# required CI gate are implemented; completion is reflected in GOAP_STATE.md.
```

Dependency reading, so a later session does not re-derive it: #960 -> #954 -> #955 is the
release chain (a refusal that cannot name the failing gate job, a verifier that stops at the
first missing artifact, an attestation that silently degrades). #948 and #949 and #961 are
independent of everything. #951/#952/#959 are **strictly sequential**: all three edit
`.github/workflows/ci.yml` and `scripts/mutation_test.sh` in the same regions (job
`mutation-test`, the `--exclude` block, `MUTATION_THRESHOLD`), and parallel PRs there
guarantee a conflict and a bot resolution that reverts one of them (`AGENTS.md` Rule 10).
`repair_the_release_gate_race` (#953) is not queued — it landed as PR #962 and is recorded as
`action_last_completed` in the 2026-11-03 archive snapshot.
