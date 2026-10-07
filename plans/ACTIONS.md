# ACTIONS — Active GOAP Action Queue

> **Compacted 2026-08-08 (ADR-0097).** This file now holds **active actions
> only** (`queued` | `in_progress` | `blocked` | `deferred`).
> Completed actions (292 entries, 2026-02 → 2026-08) are archived verbatim at
> `plans/.archive/2026-08-08-historical/ACTIONS_2026_08_08.md` (full snapshot,
> includes both completed and active sections as of the compaction date).
> Earlier history: `plans/.archive/2026-07-20-historical/` and git history.
>
> Hygiene: when an action completes, remove it here, set
> `action_last_completed` in `GOAP_STATE.md` (exactly once), and let the next
> dated reconciliation snapshot the file again. Do not re-add completed
> entries to this file.
>
> Last completed (verified 2026-10-05, sixth action):
> `gate_plan_queue_against_issue_tracker` (#851) — implemented `scripts/check-goap-queue-issues.sh`
> and `scripts/test-goap-queue-issues.sh` (50 assertions, stubbed `gh`), wired into `validate.sh` and CI
> `lint` job (`permissions: {issues: read}`, `CSM_GOAP_QUEUE_REQUIRED=true`). Asserts every queued action has an
> open `github_issue:` key, `action_last_completed` is a single key naming a removed action, and
> `queued_actions_count` matches `grep -c '^  - name:'`.
>
> Last completed (verified 2026-10-05, fifth action):
> `classify_yaml_findings_in_validate_workflows` (#841 → PR #854, squashed as `2984d3e`) —
> `scripts/validate-workflows.sh` no longer calls every yamllint finding a "YAML syntax error":
> it classifies on `yamllint -f parsable` into syntax / error-level / warning-level, `.yamllint`
> supplies the profile that makes the output actionable (`extends: relaxed`, `line-length` 200 at
> warning — priced from the tree: `ci.yml:516` is 166 columns, `benchmark-ci.yml:61` is 207), and a
> box with no parser now prints `SKIPPED — NOT A PASS` and is counted rather than passing. Two of
> the old heuristics manufactured defects instead of finding them: `grep -n '^  [^ ]' |
> grep -v '^  [a-z]'` matched line-number prefixes, and python3-without-PyYAML reported a syntax
> error for every file. 12-case fixture, wired at `validate.sh:297-303`, apt-installed in the `lint`
> job because `pip install` hits PEP 668 on ubuntu-24.04. **This entry is the gate's first catch:**
> the action was written into this file during round 8, #854 merged before the round-8 commit was
> made, `Fixes #841` closed the issue, and `scripts/check-goap-queue-issues.sh` reported it STALE on
> the uncommitted tree — the same mechanism that left #824/#827/#831/#832 alive five times today,
> now firing on its own.
>
> Last completed (verified 2026-10-05, fourth action):
> `deduplicate_unreleased_changelog_headings` (#832 → PR #848, squashed as `6ec3cf6`) —
> `## [Unreleased]` no longer holds two `### Changed` headings (`grep -n '^### '
> CHANGELOG.md` on `83e9a6c` now reads 10 Added / 14 Changed / 33 Fixed, one each), and the
> checker the issue asked for exists: `scripts/validate-changelog.sh` scores duplicated
> release-type headings per section and `scripts/test-validate-changelog.sh` (10 cases) runs
> from `scripts/validate.sh:208-214`. Draft PR **#845** was an independent implementation of
> the same issue and was closed as superseded; its CHANGELOG dedupe is the part that was kept.
> This entry is the fourth same-day instance of the drift #851 describes — #846's branch
> removed three stale entries, then #848 landed and stale-ified this one behind it — which is
> why the reconciliation gate, not another hand pass, is the fix.
>
> Last completed (verified 2026-10-05, third action):
> `cover_sigterm_in_server_shutdown` (#824 → PR #836, squashed as `2f26214`) —
> `src/shutdown.rs::operator_shutdown()` waits on SIGINT **and** SIGTERM, and both long-running
> commands now call it (`src/cli/commands/watch.rs:91`, `src/mcp/server.rs:79`), so a service
> manager's stop reaches ADR-0099's bounded reaper shutdown instead of the default disposition
> (measured on the binary beforehand: exit `0` on SIGINT, `143` on SIGTERM). No public API
> change — the primitive is `pub(crate)`, and the SIGTERM listener stays inside the non-wasm
> target table (`Cargo.toml:197-202`) that made `signal` target-gated in the first place. The
> behavior is asserted twice on purpose: subprocess exit status for the observable, and
> `src/shutdown_tests.rs` (one pending-check per arm) because the mutation profile is `--lib`
> only (`scripts/mutation_test.sh:142`) and scored **0.0000%** on head `0107c9c` with all three
> `src/shutdown.rs` mutants surviving. `Fixes #824` auto-closed the issue when the squash
> landed; nothing closed this queue entry, so the header's own hygiene rule is still unenforced.
>
> Last completed (verified 2026-10-05, second action):
> `validate_archive_manifest_completeness` (#831 → PR #838, squashed as `f235874`) —
> `plans/ARCHIVE_MANIFEST.md` went from a document no script read to one gated by
> `scripts/check-archive-manifest.sh`: **131 files under `plans/.archive/` = 131
> enumerated**, RC=0, and the manifest fails closed — it errors on a listed file missing
> from disk and on a disk file missing from the list, so the frozen archive cannot drift.
> Wired into `scripts/validate.sh:234-238`, which refuses with *"missing or not executable"*
> rather than skipping. What I verified before it merged: the tree of the squash commit is
> identical to the head I reviewed (`git diff b075a73 f235874 -- scripts plans agents-docs`
> is empty) and `shellcheck -S error` RC=0. Landing it moved `main` twice inside one round
> (dependabot `792d951`, then `f235874`), which is why #836/#829 were rebased again rather
> than pushed once.
>
> Last completed (verified 2026-10-05, first action):
> `derive_ci_crate_matrix_from_workspace` (#827 → PR #837, squashed as `87fa734`) —
> `Test Workspace Crates` is now derived from `cargo metadata` instead of a
> hand-written list guarded by a "Keep in sync with workspace members" comment. The
> observable that made it mergeable: on head `f921e81` the derived matrix produced
> **8** `Test Workspace Crates (csm-*)` jobs, all `completed/success` — the first real
> Actions execution of the `needs:` + job `outputs:` + `fromJSON` wiring, not a local
> simulation. `crates/` holds 10 members, so the 8-vs-10 gap is the whole question, and
> it is gated rather than assumed: `DEFAULT_EXCLUSIONS="csm-duckdb csm-wasm"` and the
> script **fails** if an excluded crate stops being a workspace member (`:188`) or has no
> dedicated job in `ci.yml` (`:193`). The third commit is the price of actually running
> it: `ci(ci): write the derived matrix as a single GITHUB_OUTPUT line` — a multi-line
> output value silently truncates. Cost: `scripts/ci-workspace-crate-matrix.sh` (289) +
> `scripts/test-ci-workspace-crate-matrix.sh` (290, 16 fixtures, fail-closed outside
> Actions) + `ci.yml` ±44 + `validate.sh` +15 (`:36-42`). Blind spots are recorded
> in-repo by `b394cdf` — it proves the argument appears, not that the job runs, and
> members added outside `crates/*` (`.`, `benchmarks`) are not candidates.
>
> Last completed (verified 2026-10-04, second action):
> `revive_dead_cli_parity_help_test` (#822, `55a9fa3`) — `cli_each_subcommand_has_help`
> existed in `tests/cli_parity.rs` with a body and no `#[test]`, so the harness never
> registered it: `cargo test --test cli_parity --features cli -- --list` counted 2 tests in
> a file holding 3 test bodies. No Rust sensor can see that class of defect — a `--test`
> build emits **no** `dead_code` warning for a private `fn` whose only caller would be the
> harness, so neither clippy nor the `cargo test --no-run` warning scan in `validate.sh`
> flags it. The detector is therefore textual: `scripts/check-test-attributes.sh` (187 LOC)
> reports a column-0 `fn` whose contiguous attribute block neither registers nor parks it
> and whose name no other non-comment line in the file references, exempting `pub`
> functions in `tests/<dir>/` sub-modules whose call sites live in other targets, with its
> own blind spots (string/macro mentions, nested `mod tests`, "should be a test" vs dead
> helper) stated in the header so nobody trusts it past what it measures. The revived test
> derives its oracle from the clap tree via `required_help_tokens()`: it proves every leaf
> answers `--help` and that declared flags appear in what `--help` prints, not that the
> prose is correct, and `hide(true)` args are invisible to it by design. `validate.sh` runs
> the gate *and* its 9-case `mktemp -d` fixture in the `lint` job, and both scripts are
> committed `100755`, so the `-x` guard is not a latent pass. Ordering fact worth keeping:
> the gate fails against `main` at `cli_parity.rs:97` — the very dead body this PR deletes
> — so gate and fix could not be split into two PRs.
>
> Last completed (verified 2026-10-04, first action):
> `give_the_sse_transport_an_exit_path` (#821, `dc7cd9a`) — `serve_with_shutdown(config,
> shutdown_signal)` is the new public entry point; `serve` passes `ctrl_c_shutdown()` and
> the SSE arm of `serve_with_handler` binds the listener, then runs
> `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal)`. Before this the SSE
> transport could not return: `Serve::into_future` is an unbounded accept loop (axum 0.7.9,
> `serve.rs:205-240`) and `tcp_accept` collapses errors into `Option` (`:474-496`), so
> `with_graceful_shutdown` is the only exit — which made ADR-0099's `handler.shutdown()` hop
> unreachable on the transport most users actually run. `src/mcp/server_tests.rs` (176 LOC)
> proves it by **effect**: a handler whose framework slot holds a live reaper, assert the
> cleanup `JoinHandle` slot is `Some`, resolve the signal, **await** the transport, assert
> the slot is `None` — disabling the hop turns the test red rather than leaving it
> green-by-echo. `sse_exit_is_ok_without_a_request` and
> `serve_with_shutdown_returns_when_the_signal_resolves` cover the no-request and no-handler
> paths, and `tests/mcp_sse_integration.rs` replaced its `abort()` with a `oneshot`.
> Measured against `./target/debug/csm mcp serve`: SIGINT → exit `0`, SIGTERM → `143`
> (default disposition, no cooperative stop), so the SIGTERM half is queued as
> `cover_sigterm_in_server_shutdown`. The branch went red once on `lint` for a stale
> generated file — `llms.txt` / `llms-full.txt` are gated by `scripts/check-llms-sync.sh`
> (`validate.sh:82`), so a new `pub use` must be regenerated in the same branch; the miss
> was my own skipped local gate, not CI's.
>
> Last completed (verified 2026-10-03, third action):
> `enable_ttl_cleanup_in_long_running_commands` (#817) — `csm watch` and
> `csm mcp serve` take `--ttl-cleanup-interval <SECONDS>`, default `0` so no
> long-running server starts deleting rows on its own. The value reaches
> `spawn_cleanup_task` through `create_framework_advanced`'s new
> `ttl_cleanup_interval_seconds` parameter (plus `create_framework_with_ttl`) and
> `McpConfig::ttl_cleanup_interval` → `McpHandler::with_ttl_cleanup_interval`; the
> three ordinary wrappers pass `0`. `src/cli/args.rs` hit the 500-LOC gate so the
> tail subcommand structs moved to `src/cli/args_commands.rs` and are re-exported.
> The reaper tests assert on **raw stored rows** (`Singularity::len("_default")`)
> going 1 → 0 with no purge call, and the disabled case asserts a row that
> `is_expired()` is still stored — probe-time filtering cannot explain either, so
> the test proves the task ran rather than that a field was set. Same PR fixes the
> docs deploy: 6 rustdoc `unresolved link to ':model'` warnings from one
> `args.rs` doc line had failed four main pushes while CI stayed green, because
> nothing but `GitHub Pages` (which inherits `CARGO_BUILD_WARNINGS=deny`) ever ran
> `cargo doc`; that command is now a `scripts/validate.sh` stage.
>
> Last completed (verified 2026-10-03, second action):
> `collapse_duplicate_concept_builder` — the second `ConceptBuilder`
> (`crates/csm-memory/src/singularity_types.rs`, generic over an `H` nothing ever
> instantiated) is deleted; `csm_memory::singularity::ConceptBuilder` is now a
> `pub use` of the owner, so all four public paths resolve to one type and the
> change is not breaking. Production bound the *copy*
> (`src/framework.rs`, `src/framework_ttl.rs`, `src/framework_ops.rs`), which is
> how an unclamped `now + ttl` expiry stayed live on every path while the owner
> already clamped — #806 clamped the copy and left the duplication that caused
> it. Deltas disclosed in the changelog: no `Clone` derive (zero builder `.clone()`
> call sites) and `with_metadata` now takes `impl Serialize` with ADR-0012 error
> capture on every path. Two `tests/arch_fitness.rs` guards hold the shape: one
> struct definition workspace-wide, and five public spellings passed to a fn that
> takes the owner type.
>
> Last completed (verified 2026-10-03):
> `wire_graceful_shutdown_into_servers` (#813, `f999998`) — ADR-0099's bounded
> `shutdown()` now runs on both long-running exit paths: `csm watch` handles
> Ctrl+C (previously the default SIGINT disposition killed it without flushing
> the buffered writer) and `mcp::serve` shuts the handler's framework down
> after the transport ends, so each stops the shared TTL cleanup task
> deterministically rather than through `Drop`. MCP SSE sessions now share one
> framework behind an `Arc`; `McpHandler::clone` is removed because cloning
> reset the `OnceCell`, giving every session its own framework *and* its own
> cleanup task that nobody awaited. Snapshot before this:
> `trigger_wasm_job_on_root_src_changes` (#809, `77a47d4`).
>
> Last completed (verified 2026-09-17):
> `clear_dependabot_queue_2026_09_17` (no ADR; supply-chain hygiene) — merged
> the WASM bincode fix (#725) plus the whole Dependabot backlog: consolidated
> batch PR #726 (`fuzz/Cargo.lock`, `taiki-e/install-action` 2.87.12,
> dependabot double-scope fix) superseding #717–#722, native #727
> (`clap_complete`), and two real migrations instead of ignores — rmcp
> 2.2 → 3.4 (#728: MRTR response enums, `with_all_items`, `ServerConfig`) and
> opentelemetry 0.27 → 0.32 (#729: clears GHSA-w9wp-h8wv-79jx, medium;
> `SdkTracerProvider`, `Resource::builder`). Root cause of the recurring
> `commitlint` failures: `prefix: "chore(deps)"` + `include: "scope"` emits
> `chore(deps)(deps):`, and the PR-title guard keyed on `github.actor` stopped
> skipping the bot after a maintainer ran `update-branch`. Zero open PRs and
> zero open issues after #730; 3 alerts remain non-actionable
> (`lru` GHSA-rhfx-m35p-ff5j pinned at `^0.12` by quinn-proto/tantivy,
> `libsql-sqlite3-parser` ×2 with no patched release). The three P2/P3
> actions below stay queued; `action_last_completed` unchanged because no
> queued GOAP action was in scope.
>
> Last completed (verified 2026-09-15, wave 2):
> `deduplicate_persistence_owner_bodies` (ADR-0094) — phase 1 (crate parity) was
> already on main; this wave promoted the bridge types to `csm-traits`
> (`ConceptGraph`, PR #711), moved the canonical graph CRUD into
> `csm-persistence` behind a delegating root facade (PR #711), converged the
> export payloads onto the owner-neutral `csm-traits` types (PR #713) and
> dropped the last dead copy in `csm-persistence` (this PR). Root keeps no
> second body of persistence or export payloads — only re-exports, one-line
> delegations and infallible converters; the payload schema is unchanged
> (field-level decode of pre/post bincode payloads identical except the
> wall-clock field and pre-existing map ordering; cross-imports work both
> ways). Partial progress on
> `deduplicate_test_and_source_surfaces` (P3) — action stays queued for the
> remaining root/crate test-body duplicates.
>
> Last completed (verified 2026-09-15):
> `triage_pr_roast_2026_09_15` — 0 open issues, 2 open PRs. Jules draft
> #706 (`perf(retrieval)` hybrid merge pass) closed as no-impact after
> allocator-level measurement (identical outputs, identical allocation counts
> and bytes, timing deltas opposite in sign); dependabot #707
> (`taiki-e/install-action` 2.87.10 → 2.87.11 SHA pin) verified green and kept
> for manual merge. Record: `plans/PR_ROAST_2026_09_15.md`. Same session queued
> the 0.3.8 version-truth repair, artifact hygiene, the `probe_with_rerankers`
> restoration, and the user-approved ADR-0094 persistence dedup phases.
>
> Last completed (verified 2026-09-13, wave 3):
> `dedupe_hybrid_root_shim_2026_09_13` — PR #704 merged (`9429a07`): root
> `src/retrieval/hybrid.rs` is now a 9-line re-export shim over
> `csm_retrieval::hybrid` (bm25/rerank pattern), `compute_weights` exported
> from the crate (was unreachable — the dead-copy trap #689 hit, found before
> it re-broke the CLI), root `hybrid_tests.rs` and the orphaned
> `src/retrieval/bm25/tests.rs` from #647 deleted (≈ −550 LOC). Hybrid has one
> compiled implementation; the dual-surface bug class from
> `plans/PR_ROAST_2026_09_12.md` is structurally closed. Partial progress on
> `deduplicate_test_and_source_surfaces` (P3) — action stays queued for the
> remaining root/crate test-body duplicates. mutation-test + miri verified
> green under setup-rust-toolchain v2.0.0.
>
> Last completed (verified 2026-09-13, wave 2):
> `repair_setup_rust_toolchain_2_0_0_warnings_cascade_2026_09_13` — merged the
> weekly dependabot batch (#694–#698, incl. `setup-rust-toolchain` 2.0.0) and
> repaired its fallout: v2.0.0 moved deny-warnings from `RUSTFLAGS` to cargo
> config (`CARGO_BUILD_WARNINGS=deny`), which `RUSTFLAGS`-overriding builds
> (cargo-fuzz `--cfg fuzzing`, `cargo miri setup` sysroot) cannot escape —
> fixed with `CARGO_BUILD_WARNINGS=allow` on those jobs (PRs #701/#702; valid
> levels are warn/allow/deny, "none" is rejected, key respected only by cargo
> ≥ 1.97). PR #700 merged: static `ENV_TEST_LOCK` serializes env mutation in
> embedding provider tests (intermittent voyage-test failure, run
> 103702134133). Fuzz re-dispatched on main to re-validate the nightly path.
>
> Last completed (verified 2026-09-13):
> `align_hybrid_dual_surface_selection_2026_09_13` — PR #693 merged
> (`cda3c26`): 0-based `top_k - 1` selection + `let nth` sibling style applied
> to BOTH hybrid copies (root `src/retrieval/hybrid.rs` — the copy the CLI
> runs — and `crates/csm-retrieval/src/hybrid.rs`), truthful boundary-test
> rationale in both test copies, `len == top_k + 1` mutation-kill tests, and
> documented excludes for the two equivalent mutants. PR #689 closed as
> superseded. PR #692 merged: dependabot cargo ignore for upstream-blocked
> `libsql-sqlite3-parser` (GHSA-8m95-fffc-h4c5, no patched release; weekly
> security job stopped erroring). First scheduled fuzz-full green since
> 08-02 (run 34746887403).
>
> Last completed (verified 2026-09-12):
> `fix_chronic_main_ci_failures_2026_09_12` — PR #690 merged (`0e569b7`):
> fuzz jobs now run nightly via ci.yml's pinned setup action (nix shell
> dropped; the scheduled full run had been red six consecutive Sundays
> 08-02 → 09-06; workflow_dispatch validation 34709772141 green) and Release
> `wait-for-ci` ceiling raised 1800s → 2700s / job timeout 55m (8 timeouts
> with CI green, 09-07 → 09-09). PR #689 roasted — fixes the dead
> `crates/csm-retrieval` copy of `merge_results` with no bench evidence and a
> stale boundary-test rationale; review comment posted, changes requested.
> Record: `plans/PR_ROAST_2026_09_12.md`.
>
> Last completed (verified 2026-09-07):
> `resolve_open_github_issues_and_pr_triage` — Closed superseded duplicate
> PR #672; verified and squashed PR #647 implementing all 4 open GitHub issues
> (#639, #640, #641, #642); deduplicated root retrieval modules to re-export shims
> (-5,300+ total repo LOC); updated PR titles (#670, #660); verified Codacy pass on all PRs.
>
> Last completed (verified 2026-08-12):
> `reconcile_pr_wave_2026_08_12` — PR roast wave landed
> (#620/#621/#622), BM25 absence wired, wave-33 flags trued,
> csm-cli/csm-wasm dead dupes deleted, bench harnesses shipped.
>
> Last completed (verified 2026-08-11):
> `replace_persistence_disabled_noops` (ADR-0094) — persistence-disabled
> configuration can no longer return false success: `with_local_db`/`with_turso`
> in no-persistence builds record config and `build()` rejects with
> `UnsupportedOperation`; the no-persistence `persistence` module methods all
> return `UnsupportedOperation`; un-gated persistence/CLI integration tests and
> examples were feature-gated so the full `--no-default-features` matrix compiles
> and passes. `--no-default-features --features cli` keeps compiling (ADR-0067).
>
> Last completed (verified 2026-08-08):
> `enforce_workspace_feature_contracts` (ADR-0094) — owner deps
> `default-features = false`, `persistence`/`parallel` forward explicitly,
> rayon optional in csm-memory, workspace MSRV 1.88 single-sourced;
> `cargo tree --no-default-features` has no libsql/rayon.
> Earlier same-day completions (ADR-0097): `harden_public_f32_api_validation`
> (PR #607), `recover_v037_failed_deployments` (v0.3.7 + v0.3.8 on crates.io).

> Last completed (verified 2026-09-23):
> `enforce_perf_claim_evidence_gate` — fail-closed CI check for the
> #737/#739/#740/#754 class: `scripts/check-perf-pr-evidence.py` runs in the
> always-on `commitlint` job of `ci.yml` for every PR and, for `perf(...)`
> titles, requires a `## Performance Evidence` section (new PR template
> section) naming a `plans/evidence/bench/canonical.json` benchmark id (or the
> nearest canonical comparator when none exists), a path/URL-shaped Criterion
> output or flamegraph reference (a bare `flamegraph` word does not count), and
> a numeric before/after pair; title/body arrive as argv with environment
> fallback, scope validity stays with commitlint. Nineteen-case local matrix
> exercised (non-perf/docs/bot/empty titles pass; missing section, baseline,
> artifact, numbers, placeholder-only, untouched template and "template plus
> numbers only" bodies fail; complete Criterion/flamegraph and argv
> invocations pass). Syntactic presence/shape only — it does not validate
> provenance or assert improvement. Prerequisite `fix(core)` lint repair (#759,
> merged `e5d039e`) unblocked the workspace-wide clippy sensor of the mandatory
> local harness gate, and the `lint` CI job gained the test job's RAM-backed
> `/tmp` mount (same persistence-latency gate, previously red with zero diff).
> `validated: false` and `reconcile_wave_32_remainder_and_flag_truth` stand:
> the July audit has other exit criteria.

> Last completed (verified 2026-09-18):
> `triage_pr_roast_2026_09_18` — two open PRs. #735 (`fix(framework)`: GraphRAG
> config validation) verified against the owner crate's `MAX_TRAVERSAL_DEPTH`,
> all entry points and the existing tests; two non-blocking nits; keeper, merge
> after `update-branch`. #737 (`perf(memory)`: LSH hashing + candidate scoring)
> closed as a no-op — the branch changed zero files, and its two ideas were
> adjudicated with allocator counts and the existing ANN evidence so a
> resubmission has a target to beat. Record: `plans/PR_ROAST_2026_09_18.md`.
>
> Last completed (verified 2026-09-17, evidence wave):
> `add_ann_and_persistence_scale_benchmarks` + `replace_formula_only_memory_claim`
> (ADR-0095) — the scale-evidence runner (PR #733) measures exact/HNSW/LSH/
> bucketed retrieval at 1 k/10 k/50 k, persistence throughput and contention, and
> a bytes-per-concept model with a held-out point; artifacts and manifests live
> in `plans/evidence/scale_2026_09_17/`. The persistence contention half was red
> before the fix (0.905 error rate with eight writers) and green after PR #734
> added a 5 s busy timeout plus bounded retries to `csm-persistence`
> (0.000 error rate, same workload, before/after artifacts side by side). The
> memory claim is now measured, not asserted: 4 691 B RSS and 2 850 B persisted
> per concept (held-out error 0.29 % / 0.06 % at 100 k), so the recorded
> `< 12 MB for 10M concepts` target — which described the unimplemented ADR-0024
> phase-2 product quantization — is marked not supported in the book and
> `docs/architecture/context.yaml`, and the formula-only test was replaced by a
> measured one. `deduplicate_test_and_source_surfaces` (P3) stays queued.

> Last completed (verified 2026-09-20):
> `triage_pr_roast_2026_09_20` — two Jules perf PRs (#739, #740) against the same
> `merge_single_list` hunk in `crates/csm-retrieval/src/hybrid.rs`, both replacing
> a `TrustedLen` `collect()` with `with_capacity` + push: allocator counting showed
> byte-identical allocation profiles (102/6 090 B at n=100, 1 002/61 890 B at
> n=1000, both variants), so both were roasted and closed as no-ops; #740's
> 13.67 % claim had no criterion output. #735 from the 2026-09-18 wave stays
> merge-ready. Record: `plans/PR_ROAST_2026_09_20.md`.
>
> Last completed (verified 2026-09-18, test-surface dedup):
> `deduplicate_test_and_source_surfaces` (ADR-0094, ADR-0095) — removed the 24
> facade test bodies that were byte-identical duplicates of owner-crate tests
> (12 in `src/embedding/mod.rs`, 6 in `src/persistence_wasm.rs`, 3 with the
> `csm-core-lib` neural-circuit copy, plus three singles), turned
> `csm-core-lib::maps::neural_circuit` into a re-export of the `csm-chaos`
> owner (its feature was enabled by nobody, so the second body was
> unreachable), and ported the `ENV_TEST_LOCK` race fix into `csm-embedding`
> before deleting the facade copies that held it. Six further "duplicate" files
> were *rejected* by mechanical body comparison and hand review — they assert
> distinct behavior (metrics vs membership, error variants, retrieval after
> TTL) and stay. Coverage methodology replaced: `scripts/update-coverage.sh`
> (test-LOC ratio, `crates/` blind) → `scripts/coverage-report.sh`
> (unique compiled behavior; llvm-cov line/branch: 74.33 % lines / 64.08 %
> branches over lib + integration targets). Audit: `plans/TEST_SURFACE_AUDIT_2026_09_18.md`.

> Last completed (verified 2026-09-21):
> `fix_bucketed_candidate_recall_at_scale` (ADR-0095 evidence → ADR-0065 path) —
> the bucketed candidate generator no longer loses recall as the corpus grows:
> the probe width scales with N (`effective_bucket_probe_width`, configured
> width as a floor, capped at `MAX_BUCKET_PROBE_WIDTH`), candidates within one
> masked bit of the query bucket are accepted (10 k: recall@10 0.604 → 0.832 at
> floor 8, 249 µs vs 558 µs exact), and an oversized probe now returns nothing
> so the caller scans exactly instead of slicing the bucket by index order — the
> failure that measured 0.016 recall at floor 2. Evidence and the sweep table:
> `plans/evidence/scale_release_2026_09_21/` (`bucket_sweep/` holds pre- and
> post-fix runs at 10 k/50 k/200 k for floors 2 and 8).

> Last completed (verified 2026-09-22):
> `triage_pr_roast_2026_09_22` — one open PR. #754 (`perf(memory)`: pre-allocate
> capacity + drop iterator closures in `singularity_retrieval`) closed as no
> demonstrated impact: counting-allocator adjudication showed hunk 4 (both scan
> tails) byte-identical (257 allocs / 11 264 B — `TrustedLen` `collect()` is
> already single-exact-alloc), hunk 2 self-contradicting (`with_capacity(len.min(max))`
> then pushing `len` before `truncate`), and the remaining ~13-alloc growth-chain
> delta dwarfed by the 257 `String` clones the rewrite keeps. No criterion
> evidence attached (4th entry in the #737/#739/#740 class). Record:
> `plans/PR_ROAST_2026_09_22.md`. Five followups queued below; `action_last_completed`
> unchanged because no queued GOAP action was in scope.

> Last completed (verified 2026-09-30):
> `triage_pr_roast_2026_09_30` — drained the open queue: #791 (owner-crate cache
> invalidation + capacity-safe upsert) merged as `cd70fa5`, #789 (2026-09-28 roast
> record + skill lesson) as `977f36f`, and #792 (2026-09-29 recommendations +
> `plans/PR_ROAST_2026_09_30.md`) as `195d6e61`; #790/#793 closed with roast
> comments for selecting the wrong candidate under negative public weights, and
> #783 closed earlier as zero-delta. Every keeper was rebased onto the current
> `main`, commitlint-validated, and re-checked on the exact new head before an
> explicit squash merge — never `gh pr merge --auto`. Records:
> `plans/PR_ROAST_2026_09_28.md`, `plans/PR_ROAST_2026_09_29.md`,
> `plans/PR_ROAST_2026_09_30.md`. 0 open PRs, 0 open issues; the three queued
> actions below are unchanged (none was in scope).

> Last completed (verified 2026-09-30, wave-32 exit pass):
> `reconcile_wave_32_remainder_and_flag_truth` — re-verified every exit
> criterion of `plans/GOAP_AUDIT_2026_07_14.md` against HEAD `7ce6fb73` instead
> of trusting recorded flags. Met: ANN config fallible + revisioned,
> fingerprint-gated snapshots (`tests/ann_revision_envelope.rs`); fuzz
> workspace compiles (exact CI command, exit 0); lean `--no-default-features`
> graph (0 libsql/rayon on normal+build edges); one WASM artifact (CI + release
> both `build-wasm.sh release-web`); full workspace/supply-chain CI; bulk
> association load; benchmark metric math + hand-calculated tests; measured
> memory claim; absence short-circuit wired; 0 TODO/`unimplemented!`; unique
> source/test ownership; compact plans. Remainder queued as nine actions (TTL
> shutdown ownership, absence invalidation, failure-path test, query-count
> test, gate/fixture/hook consolidation, skill catalog + agent-context
> generation, scheduled/release evidence tiers + mutation inventory,
> machine-derived CI matrix, archive-manifest validation). `validated` stays
> false with that evidence-based justification; `wave_32_status` stays
> in_progress.

> Last completed (verified 2026-09-30, release gate):
> `migrate_release_wait_for_ci_to_workflow_run` — PR #796 merged as
> `8c1f864d`: `release.yml` triggers on the CI workflow completing on `main`
> (`workflow_run`) instead of a push plus a 45-minute poll, releases exactly
> `github.event.workflow_run.head_sha`, gates the job on
> `conclusion == 'success'` + same-repo + `event == 'push'`, and keeps
> `workflow_dispatch` fail-closed with a single CI query. Proof: trigger run
> 36746741481 (`validate` success, every publish job skipped, and no
> push-triggered release run for the merge SHA) plus dispatch run 36746882951
> (log `CI on 8c1f864d…: status=completed conclusion=success`, then
> `Tag v0.3.8 already exists; skipping release.`).

> Last completed (verified 2026-09-30, publish pre-check):
> `fix_crates_publish_precheck_and_add_duckdb` — PR #798 merged as
> `71a48618`: the name pre-check is ownership-based (owners API; free or
> `d-o-hub` passes, other owners fail, unknown HTTP fails closed), version
> existence comes from the sparse index (fail-closed if unreadable), the
> companion loop classifies failures instead of swallowing them, and
> `csm-duckdb` has a dedicated step after the root publish that waits for the
> root version in the index. Verified by executing the workflow's extracted
> shell: eight-name list green, `serde` conflict red, `0.3.8`/`0.3.9`
> published-probe split correct, duckdb step no-ops on the published version.

> Last completed (verified 2026-10-01, TTL lifecycle):
> `own_ttl_cleanup_shutdown` — PR #801 merged as `96144703` (ADR-0099):
> cooperative `watch` cancellation replaces `Drop`-time `abort()`, the loop
> also stops when the last handle drops, `shutdown()` cancels and awaits under
> a 5 s bound (idempotent, shared across clones, no-op on wasm32), and two
> CI iterations were root-caused on the way — the 500-line LOC gate via
> child-module extraction (`src/framework_cleanup.rs`) and three missed
> mutation mutants via in-crate unit tests, not exclusions. Verification:
> 20 TTL integration tests + 2 unit tests + 6 arch-fitness tests pass locally,
> and CI on `49b6a74` has lint, test, mutation-test, miri, workspace crates,
> benchmark-small and commitlint green.

> Last completed (verified 2026-10-01, triage):
> `triage_pr_roast_2026_10_01` — one open PR. #800 (`perf(core)`: zero-shift
> fast path for `HVec10240::permute`, draft) kept open with an evidence
> request: correct (byte-identical for `shift ≡ 0 (mod 10240)`, locked by
> `hyperdim_tests.rs:48`) and reachable (`encoder.rs:226` calls `permute(0)`
> for position 0), but `commitlint` is red on the perf gate (`missing
> '## Performance Evidence' section in the PR body`) and the only `permute`
> bench uses shift `321`, which cannot enter the new branch. Record:
> `plans/PR_ROAST_2026_10_01.md`; lesson distilled into
> `.agents/skills/pr-roast-triage/SKILL.md`. No queued GOAP action was in
> scope; counters unchanged.

> Last completed (verified 2026-10-01, absence invalidation):
> `add_absence_invalidation_semantics` — PR #804 merged as `c8f91876`: absence
> rows now carry namespace + namespace_revision, so any durable mutation makes
> them stale (no write on the mutation path), successful probes delete the
> record they found, and the attempt counter restarts when the revision moved.
> Migration v12 adds the columns with ('', 0) defaults that keep old rows inert.
> Verified by a new regression (fails with `got ["AbsenceShortCircuit"]` when the
> revision comparison is removed), a parked-v11 upgrade check, four root-caused
> CI iterations ending at a 100% mutation score, and a green CI on `abff706`
> (lint, test, mutation-test, miri, nine workspace crates, deny, commitlint).

> Last completed (verified 2026-10-02, CI repair):
> `trigger_wasm_job_on_root_src_changes` — PR #809 merged as `77a47d4`: the
> `wasm` job's type-freshness check filtered only the pre-hash closure-helper
> names, so the crate-hash-prefixed declarations wasm-bindgen now emits
> (`wasm_bindgen_847cf8bb5373c819___convert__closures…`) were reported as API
> drift and `main` went red on a diff of exactly three compiler-generated
> closure helpers (runs 36885556956 and 36888300162). The filter is now
> anchored at the declaration start and matches only generated
> closure-invocation declarations — both naming schemes — while public exports,
> `InitOutput` members and `__wbindgen_*` ABI signatures are still compared and
> the checked-in snapshot was deliberately not refreshed. The same PR widens the
> job's PR path filter to root `src/**`, the WASM build/check/size-gate scripts
> and `ci.yml` itself, so the #804 class of root-source change exercises the gate
> before merge instead of exposing itself on the trunk. Verification: the
> checker's own `filter_dts` accepts the job-log-derived helper-only fixture
> (the previous filter rejects it) and still rejects four public-API/ABI
> mutations; a twelve-case path-filter matrix; local end-to-end — freshness
> `OK`, `release-web` package 655 497 B, size gate pass, `wasm/test.js` prints
> `WASM smoke test passed.`; PR #809's `wasm` job green on head `d0536879`
> (`Verify WASM TS Freshness` → `OK`) and main push run 36983336327 green.
> Eight actions remain; `validated` stays false.

actions:
  - name: wire_and_consolidate_validation_gates
    github_issue: "#829"
    status: in_progress
    preconditions: []
    effects:
      single_gate_graph: true
    notes: >
      2026-10-05 EFFECTS REVISED: the two keys this action used to advertise —
      `fixture_suites_wired_into_validate` and
      `skill_format_gate_fixtured_and_fail_closed` — are achieved on `main` as of
      `77bd5f0` (PR #846), so they moved to `GOAP_STATE.md` as `true` and stopped
      being claimed here. A queued action advertising an effect the state file
      already reports true is precisely the #851 drift, and
      `scripts/check-goap-queue-issues.sh` now fails on it. What stays queued is
      the parent claim: one canonical gate graph. That remainder decomposed into
      #840 (link validators), #841 (yamllint profile + severity
      classification, landed as PR #854 / `2984d3e`) and #842 (hook installers), each now carrying its own
      action block below.
      ----
      Audit W1/G3, re-verified 2026-09-30 and again 2026-10-05 (issue #829 comment).
      DONE on the branch: `validate-skill-format.sh` fail-open closed (an unclosed
      frontmatter block validated) and given its first fixture; `test-llms-sync.sh`,
      `test-version-sync.sh` (SCRIPT_DIR-resolved, exec bit restored) and the new
      `test-validate-skill-format.sh` wired into `validate.sh`, hence into the CI
      `lint` job at `ci.yml:587`; `negative-fixtures.sh` deleted — it asserted that
      rustc rejects bad syntax and rustfmt flags unformatted code, i.e. the toolchain,
      not this repo. NOT DONE, each split to its own issue because "just wire it" is
      wrong for all three: #840 two competing link validators, both unwired, one of
      which resolves root-relative `@imports` against the skill directory; #841
      `validate-workflows.sh` reporting yamllint lint noise as "YAML syntax error"
      with no `.yamllint` profile in the repo; #842 three hook installers with three
      different hook sets and `.githooks/` never installed by any of them. Original
      note preserved below for provenance.
      ----
      Audit W1/G3, re-verified 2026-09-30. Negative fixtures exist
      (`scripts/test-llms-sync.sh` 5 cases, `test-version-sync.sh` 4 cases,
      `negative-fixtures.sh`) but no gate invokes them;
      `validate-workflows.sh`, `validate-git-hooks.sh` and `validate-links.sh`
      have zero callers; three hook installers (`install-hooks.sh`,
      `setup-hooks.sh`, `validate-git-hooks.sh --install`) install different
      hook sets; `pre-commit.sh` and `hooks/pre-push` enforce different sensors
      than `harness-check.sh`; the CI-wired skill validator has no negative
      fixture. Fresh measured evidence (2026-10-05): `scripts/validate-links.sh`
      is red on `main` right now — 5 broken refs (`@AGENTS.md` in
      `jules-orchestration` and `rust-development`, `@file.md` in
      `skill-creator`, `@file.md` + `./path.md` in `testing-validation`) — and
      nothing invokes it, so a gate that fails is contributing zero signal.
      Required: one bootstrap, one canonical gate graph, fixture tests wired into
      `validate.sh` + CI (or deleted), negative fixture for the skill validator.
      2026-10-05 STATUS: PR **#846** merged as `77bd5f0` — three of the six orphan fixtures wired there, the skill-format fail-open fixed with a 15-case fixture, `negative-fixtures.sh` deleted. The remaining gates are deliberately NOT in this action: #840 (link validators), #841 (yamllint profile), #842 (hook installers). Do not re-implement #846's work elsewhere. Re-measured on `83e9a6c` for the remainder: `scripts/validate-links.sh` reports 5 broken links and exits 1, `scripts/check-docs-links.sh` reports 39 issues and exits 1, and `scripts/validate-git-hooks.sh` / `scripts/validate-workflows.sh` still have zero callers in `validate.sh` or any workflow — three red gates and two unwired gates contributing no signal.

  - name: generate_skill_catalog_and_agent_context
    github_issue: "#828"
    status: in_progress
    preconditions: []
    effects:
      skill_catalog_generated_and_gated: true
    notes: >
      Audit Phase-4 exit, re-verified 2026-09-30. `.agents/skills/CATALOG.md`
      says "32 skills" while the disk has 33, with no generator and no drift
      check in `scripts/`, `.github/` or hooks. Same family:
      `scripts/gen-agents-context.sh:77` hardcodes "Skills (13 Total)" and
      `:13` reads archived `plans/SWARM_COORDINATION.md`;
      `docs/architecture/context.yaml` is stale (`current_status.active_wave:
      11`, `total_tests: 134`, `skills.total_count: 19`, `adrs.count: 16`) and
      its `architecture.modules` names pre-extraction root files. Required:
      generator + drift gate for the catalog, and refreshed (or generated)
      agent-context artifacts with a checker.
      2026-10-05 STATUS: PR **#847** merged as `6d936ee` — `scripts/gen-skill-catalog.sh` +
      `scripts/check-skill-catalog.sh` (76 lines) + a 23-assertion drift fixture, `CATALOG.md`
      regenerated to 33 skills and proven byte-identical on re-run, wired at `validate.sh:235-241`
      (fail-closed `else` at `:241`) and `ci.yml:598`. Its seven Codacy SC2016 findings cleared
      honestly: two removed (`render_catalog` rewritten as one quoted heredoc, regeneration proven
      byte-identical), five literal-backtick lines given `# shellcheck disable=SC2016` with a reason.
      **This entry stays queued on purpose.** The action names two artifacts and #847 shipped one:
      `scripts/gen-agents-context.sh:77` still hardcodes "Skills (13 Total)" into the drawio XML,
      `docs/architecture/context.yaml` still carries `skills.total_count: 19`, which
      `scripts/yaml-to-drawio.py` renders, and nothing checks either. Flipping
      `skill_catalog_generated_and_gated` to `true` while the action is still queued is exactly what
      `check-goap-queue-issues.sh` errors on, so the effect waits for the half that is not done.

  - name: complete_evidence_tiers_and_mutation_hardening
    github_issue: "#830"
    preconditions: []
    effects:
      scheduled_and_release_evidence_tiers: true
      mutation_inventory_published: true
    notes: >
      Audit E4/E6 + ADR-0095 tiers, re-verified 2026-09-30. The PR tier exists
      (`benchmark-ci.yml`; `ci.yml` test-benchmarks, graph-candidates,
      perf-evidence gate), but no scheduled workflow runs the scale evidence
      (`scripts/scale-evidence.sh`) or a full mutation sweep;
      `pre-release-gate.yml` (workflow_call + workflow_dispatch) has no caller
      and runs no benchmark; `benchmark-ci.yml` path filters omit `crates/**`,
      so owner-crate changes skip it. Mutation: timeouts are unresolved ✓ and a
      budget fails the job ✓, but the static exclude list
      (`scripts/mutation_test.sh:181-262`) still applies to changed files and
      only aggregate counts are printed (no module-level inventory artifact).

  - name: add_query_count_regression_test
    github_issue: "#826"
    preconditions: []
    effects:
      bulk_association_load_verified: true
    notes: >
      Audit P1 remainder, re-verified 2026-09-30; re-measured 2026-10-03 (read-only
      Explore pass, spot-checked). `load_all_associations`
      (`crates/csm-persistence/src/persistence_index.rs:149` — the note's `:148` is the
      doc comment, stale by one) returns `Result<Vec<(String, String, f32, u64)>>` in one
      query at `:153-158`, and is used by `load_replace`/`load_merge`/
      `reload_namespace_from_rows` (`src/framework_persistence.rs:100,184,304` — exact),
      so the N+1 loop is gone, but nothing counts queries: `grep -rn "query_count|num_queries"`
      = 0 hits repo-wide (verified). COST CLASS IS `fix(...)`, not `test(...)` — pinning a
      query count requires production instrumentation (a counter on `Persistence`
      incremented at the query sites, or a test-only wrapper connection); there is no
      existing handle to observe. Neighbour that is NOT a duplicate:
      `persistence_index.rs:250` `bulk_associations_load` asserts correctness
      `all.len() == 1`), not query count, and the module's `#[cfg(test)] mod tests` is at
      `:188`. LOC is comfortable (`persistence_index.rs` 274/500, `persistence.rs` 385/500).
      Visibility caveat the implementer must be told: the test compiles in
      `cargo test -p csm-persistence --lib` (CI runs that at `ci.yml:240`, and the crate's
      `default = ["persistence"]` gates it via `crates/csm-persistence/src/lib.rs:8-17`),
      but the mutation fast profile does NOT include `csm-persistence` in its `-p` list
      (`scripts/mutation_test.sh:142`), so this gate is CI-visible, not mutation-visible —
      if it must be mutation-visible it also needs a root-crate `--lib` caller test.
      Smallest honest scope: cfg(test) counter + one test extending `bulk_associations_load`
      to N=50 asserting exactly one association query.

  - name: reconcile_and_wire_the_two_link_validators
    github_issue: "#840"
    preconditions: []
    effects:
      link_validators_single_and_wired: true
    notes: >
      Split out of #829, which deliberately stopped at the fixtures. Two scripts
      do overlapping work and neither has a caller: `grep -rl
      'validate-links.sh\|check-docs-links.sh' scripts/validate.sh
      .github/workflows/` returns nothing, while both are red right now —
      measured on `83e9a6c`: `scripts/validate-links.sh` reports 5 broken links
      and exits 1 (`@AGENTS.md` in `jules-orchestration` and `rust-development`,
      `@file.md` in `skill-creator`, `@file.md` + `./path.md` in
      `testing-validation`), `scripts/check-docs-links.sh` reports 39 issues and
      exits 1. Required: decide which validator owns which reference class, fix
      or exempt the 5 + 39 findings, wire the survivor into `validate.sh` behind
      the `[[ -x ]] … exit 1` guard this repo standardised in #829/#846, and give
      it a negative fixture so a neutered check cannot pass. Note the trap that
      made `validate-links.sh` useless as a sensor even if it were wired: it
      resolves root-relative `@imports` (the `AGENTS.md` convention, e.g.
      `@plans/ACTIONS.md`) against the *skill directory*, so real imports read as
      broken — fix the resolution root before believing its exit code.
      Measure exit codes without a pipe: `bash scripts/x.sh; echo $?`, never
      `bash scripts/x.sh | tail -1; echo $?`, which reports `tail`'s status and
      made a failing validator look like it exited 0.

  - name: consolidate_the_three_hook_installers
    github_issue: "#842"
    status: in_progress
    preconditions: []
    effects:
      single_hook_bootstrap_installed_by_default: true
    notes: >
      Split out of #829. Three installers produce three different hook sets:
      `install-hooks.sh` installed only `scripts/hooks/pre-push`; `setup-hooks.sh`
      installed only `scripts/pre-commit.sh` as pre-commit;
      `validate-git-hooks.sh --install` looped over `pre-push commit-msg` looking
      for `scripts/<hook>.sh` files that do not exist, so it silently installed
      pre-commit only. `.githooks/pre-commit` and `scripts/pre-commit.sh` are not
      the same file (`cmp`: differ at byte 40), and `core.hooksPath` was never set
      — measured again on `83e9a6c`: this checkout's hooks dir
      (`.git/hooks`, resolved through `git rev-parse --git-common-dir`) contains
      exactly one non-sample hook, `pre-push`, while `.githooks/` holds
      `pre-commit` and is referenced by `tooling-guard.yml:24` but installed by
      nothing. So the committed guard-rail hooks have never run on any local
      commit. 2026-10-05 STATUS: implemented and verified locally on branch
      `ci/consolidate-hook-bootstraps` at `cdf711e` — one reconciled
      `.githooks/pre-commit`, `install-hooks.sh` copying into
      `$(git rev-parse --git-common-dir)/hooks` by default (an absolute
      `core.hooksPath` under `--link`, because a relative one does not fire inside
      linked worktrees on git 2.43.0), `setup-hooks.sh` reduced to a shim,
      `validate-git-hooks.sh` fail-closed, a 19-test `test-hook-bootstrap.sh`,
      `validate.sh` wiring; not pushed yet — it goes after #847/#854 so the
      `validate.sh` region rebases once. Disclosed consequences an implementer
      must not skip: local commits start running clippy (≈3 min with a warm
      shared `CARGO_TARGET_DIR`), `scripts/pre-commit.sh` keeps doc references in
      `docs/release-guardrails.md:65,67,112,127` and `HARNESS.md:45` after no
      script references it, and `.githooks/pre-commit` cannot be deleted because
      `tooling-guard.yml:24` guards it.


  - name: mutation_baseline_the_feature_gated_mcp_module
    github_issue: "#833"
    preconditions: []
    effects:
      mcp_module_mutation_baselined: true
    notes: >
      #817 added `--features chaotic_semantic_memory/mcp` to the fast mutation
      profile, but `--in-diff` only generates mutants for lines a PR touches, so
      what that fixes going forward is not what it fixes now: `src/mcp/**` was
      invisible to *every* mutation run before it (the module is
      `#[cfg(feature = "mcp")]` and `mcp` is not a default feature), so the
      module has no mutation baseline at all. Required: run cargo-mutants over
      `src/mcp/**` with the mcp feature on (full profile, or `--file`-scoped fast
      runs per module), then triage each survivor into a real test or a documented
      `--exclude-re` entry with its mechanism. Constraints to respect: the PR job
      is `timeout-minutes: 45` with the full-tree fallback disabled in CI, so this
      is a local/nightly run; and the profile's `--build-timeout` is sized for
      incremental per-mutant rebuilds, not for a cold baseline that links
      rmcp/axum/tower (see LEARNINGS 2026-10-03). The concrete target this note
      was written about is now closed: #821 gave the SSE transport an exit, so the
      ADR-0099 hop at `src/mcp/server.rs:152-155` (`handler.shutdown()`, delegating
      to `src/mcp/handler.rs:90`) executes on both transports and is asserted for
      its effect in `src/mcp/server_tests.rs`. What remains is the baseline itself:
      `src/mcp/**` is still outside the profile via `--exclude "src/mcp/*"`,
      `--exclude-re "mcp::"` and `"McpHandler::"`, so the module has no mutation
      evidence at all — run a scoped baseline over `src/mcp` and triage each
      survivor into a real test or a documented exclusion with its mechanism.

