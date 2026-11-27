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
# Last completed 2026-11-15: patch_quinn_udp_rustsec_2026_0538 (#1034),
# merged as PR #1035 -> 11ce127d. Cargo.lock resolves quinn-udp 0.5.16;
# local cargo audit, cargo deny, and PR CI passed. Main CI run 43687783467
# failed at queue reconciliation because this closed issue remained active.
# The post-merge security audit still lacks a successful main-CI verdict.
# Last completed 2026-11-15: implement_typed_ci_path_selection_992 (#992) —
# the seven detect-changes outputs now come from crates/csm-ci-selection, so
# src/cli/mcp.rs selects mcp-feature instead of leaving that job skipped.
# Queue stays empty; the roast record is plans/PR_ROAST_2026_11_15.md.
# Queue empty as of 2026-11-16 too: the three open Dependabot PRs that held
# main's Pre-Release Gate refused (PR Drain, run 43840583406) were roasted,
# brought up to the current main one at a time and drained - #1042 -> a6dad76b,
# #1041 -> d7e10e2d, #1043 -> d1ec01bf - and the roast record landed as PR
# #1044 -> 66195f0f. Zero open PRs afterwards.
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

The queue is still empty as of 2026-11-15: `scope_mutation_build_to_lib_target_1005` landed as PR
#1039 -> `c59c752c`, closing issue #1005, with all eight mutation shards green in 3-4 minutes (shard
7 — the one that died in its baseline — 3m19s). Its predecessor,
`repair_mutation_scope_and_parser_hang_1038`, completed immediately before it. Both are recorded as
`action_last_completed` history in `plans/GOAP_STATE.md` and in git, not re-added here.

The queue is still empty as of 2026-11-16: the Dependabot trio (#1041/#1042/#1043) landed after the
roast and drain described above, and there are no open PRs at that point.

The queue is still empty as of 2026-11-24: two RustSec advisories published after `main`'s last green
run (`RUSTSEC-2026-0592` on `url`, patched in 2.5.9; `RUSTSEC-2026-0543` on `chrono`, unmaintained with
no patch) were reddening `Cargo Deny` and `lint` on every head. The fix landed first as #1057
(`65fed3ac`), then the six dependency PRs were rebased onto each successive `main` and merged one at a
time with all six required contexts green on the exact head: #1051 `1bf3fa35`, #1058 `af3fa216`
(superseding #1052, which Dependabot closed mid-flight), #1053 `71dbd8ca`, #1054 `ed4e863a`, #1055
`fd498e76`, #1056 `e00fbeb8`. Zero open PRs afterwards; record `plans/PR_ROAST_2026_11_24.md`.

The queue is still empty as of 2026-11-26: issue #1076's shape drift
(`key_files.adrs` pointing at `plans/adr/`) was roasted, implemented and recorded
inside one session, so nothing was queued between start and completion — the
entry is `action_last_completed: context_key_files_adr_registry_1076` in
`plans/GOAP_STATE.md`, the diff is PR #1083, and the verdict table is
`plans/PR_ROAST_2026_11_26.md`. Open issues afterwards: #1075 (duplicate test
names vs the 10 printed) and #1070 (owner-blocked crates.io OIDC configs).

The queue is still empty as of 2026-11-26 (second action of the day): issue #1075
was roasted, implemented and recorded inside one session, so nothing was queued
between start and completion — the entry is
`action_last_completed: duplicate_test_name_leads_1075` in `plans/GOAP_STATE.md`,
the diff is the `test/workspace-test-name-dispositions-1075` branch, and the
disposition table is the second section of `plans/PR_ROAST_2026_11_26.md`. The
three same-body duplicates are gone (1247 -> 1244 tests), the nine distinct-name
collisions are renamed per subject, and the inventory now reports 0 shared names
with a fixture that fails when its header count and its listing disagree. Open
issues afterwards: #1070 (owner-blocked crates.io OIDC configs) and the new
follow-up on `csm-chaos --features experimental-voronoi-lsh` not compiling.

The queue is still empty as of 2026-11-26 (third action): issue #1084 — filed from
the #1075 verification, so it never sat in the queue — was roasted and fixed in
the same session. `action_last_completed: fix_no_std_experimental_test_build_1084`.
Two `vec!` calls in `csm-chaos`'s `voronoi_lsh` test module became
`alloc::vec![…]`, and `scripts/validate.sh` gained a `std`-off build of the four
experimental features (the `--all-features` stages that CI's `lint` job runs had
`std` on, which is why nothing saw it). Remaining open issues: #1070
(owner-blocked crates.io OIDC UI configs — the 2026-11-17 dispatch's 400 names
`release.yml` nine times, so the workflow side needs no change).

The queue is still empty as of 2026-11-27 (second action): #1089 (`ci(release)`, the do-harness
release-pipeline port) was roasted rather than merged — required `lint` red on a 253-line
`SKILL.md`, no ADR for the trigger inversion, `release_wait_event_driven` left stale, and a 15-file
single-commit diff. The demands are on the PR itself and the verdict is
`plans/PR_ROAST_2026_11_27.md`; nothing was queued because the work belongs to that PR's author,
and #1070 stays owner-blocked on crates.io UI config.


The queue is still empty as of 2026-11-27 (third action): the #1089 roast demands were implemented on that
PR's own branch (skill trim 253 → 246, ADR-0106 + registry + supersession note in ADR-0042, the
`release_wait_event_driven` flag flipped, `release-no-verdict.sh` deleted with its fixture cases and catalog
entries). Still outstanding there: the diff split. Nothing was queued, because the remaining work is the
author's and #1070 stays owner-blocked.
