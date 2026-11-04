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
  - name: shard_the_mutation_test_context
    github_issue: 951
    preconditions:
      host_subprocesses_isolated_from_nix_env: true
    effects:
      mutation_test_sharded_behind_one_context: true
    cost: 5
    status: queued

  - name: detect_new_cfg_disabled_mutants
    github_issue: 952
    preconditions:
      mutation_test_sharded_behind_one_context: true
    effects:
      new_cfg_disabled_mutants_detected_by_inventory: true
    cost: 3
    status: queued

  - name: make_mutation_threshold_draft_invariant
    github_issue: 959
    preconditions:
      new_cfg_disabled_mutants_detected_by_inventory: true
    effects:
      mutation_threshold_independent_of_draft_flag: true
    cost: 2
    status: queued

  - name: validate_pinned_action_shas_in_ci
    github_issue: 1002
    preconditions:
      fixture_suites_wired_into_validate: true
    effects:
      github_action_pins_validated_in_ci: true
    cost: 2
    status: queued
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

A second ci.yml collision set, added 2026-11-04 from `plans/PR_ROAST_2026_11_04.md`: #1000 edits the
`mutation-test`/`mutation-shard` jobs and #1002 edits the `lint` job's `validate.sh` step, while the
re-creation of #992 centralizes **every** job's path selection in the same file. Order them
#1000 → #1002 → #992, or the third one conflicts with both. #992 is not listed above because it is
tracked as a re-created PR, not a fresh action; its acceptance criteria live in the roast record.
