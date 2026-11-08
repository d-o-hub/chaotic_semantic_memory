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
# Queue opened 2026-11-08 from the six open issues; roast verdicts in
# plans/PR_ROAST_2026_11_08_ISSUES.md. Order is the merge plan there: #1004
# unblocks main (lint is a required context), then the independent script/workflow
# fixes. #1014 is operator-only and stays blocked until crates.io access exists.

  - name: repair_the_cached_duckdb_unpack
    github_issue: 1004
    preconditions: []
    effects:
      duckdb_cache_pollution_repaired: true
    cost: 2
    status: queued

  - name: extract_the_ci_path_selector
    github_issue: 992
    preconditions: []
    effects:
      ci_path_selection_tested_and_mcp_complete: true
    cost: 5
    status: queued

  - name: resolve_hyphen_underscore_fixture_pairs
    github_issue: 1003
    preconditions: []
    effects:
      script_catalog_fixture_pairs_resolved: true
    cost: 2
    status: queued

  - name: gate_actions_sha_pins_in_ci
    github_issue: 1002
    preconditions: []
    effects:
      actions_sha_pins_validated_in_ci: true
    cost: 4
    status: queued

  - name: compile_only_the_lib_target_in_mutation_builds
    github_issue: 1005
    preconditions: []
    effects:
      mutation_build_scope_matches_scored_scope: true
    cost: 4
    status: queued

  - name: operator_publish_v0_3_9_over_oidc
    github_issue: 1014
    preconditions:
      release_pipeline_validated: true
    effects:
      crates_io_0_3_9_published_9_of_9: true
    cost: 8
    status: blocked
    # Operator-only: nine trusted-publisher configs, the recovery dispatch, and
    # revoking the old API token. The repository half is PR #1015 (ADR-0104).
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
