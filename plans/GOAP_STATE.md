# GOAP World State — canonical current state

> **Compacted 2026-08-08 (ADR-0097).** This file holds **current truth only**:
> flags an agent needs to plan the next action. Historical per-wave/PR
> narrative (2026-02 → 2026-08) is archived verbatim at
> `plans/.archive/2026-08-08-historical/GOAP_STATE_2026_08_08.md` and
> `plans/.archive/2026-07-20-historical/`; full history is also in git.
>
> Hygiene rules:
> - `action_last_completed` appears **exactly once** (last key in this file;
>   YAML last-key-wins makes earlier duplicates silently dead).
> - Dated snapshots (metrics, PR logs, wave reports) belong in the archive,
>   not here. Record only the *current* value with a short dated comment.
> - When a flag flips, update it in place; do not append a new block.

world_state:
  # ── Core project ──────────────────────────────────────────────
  project_initialized: true
  dependencies_added: true
  core_modules_created: true
  tests_passing: true
  benchmarks_exist: true
  wasm_compiles: true
  binary_built: true
  documentation_complete: true
  validated: false               # 2026-09-30: wave-32/33 exits re-verified vs GOAP_AUDIT_2026_07_14.md; residuals queued — TTL shutdown, absence invalidation, failure-path/query-count tests, gate+catalog consolidation, scheduled/release evidence tiers
  ci_all_checks_passed: true     # 2026-10-03: #817 merged as 1389c9e with CI 37139534832 green on the
                               #   landing head (27 jobs: 25 success, 2 skipped, 0 failure) incl. the
                               #   mutation-test job that had gone red at 25% on its first head c47d676
                               #   (root cause: the profile's feature set, not weak tests -- see
                               #   mutation_profile_compiles_feature_gated_mcp). The push that landed
                               #   it is green as well: CI 37141216196 (26 success, 1 skipped, 0
                               #   failure) and CodeQL 37141215898, read per workflowName from
                               #   `gh run list`. GitHub Pages run
                               #   37141216190 on that push SUCCEEDED -- the first green docs deploy
                               #   since 8bb4521 (2026-09-09) after four consecutive failures
                               #   (34751139063, 35274354770, 35614812503, 37126532546) that all kept
                               #   their required checks green; verified end-to-end, the live
                               #   ttl.html serves the new --ttl-cleanup-interval text.
                               #   Prior main push 37126532544 (4064af9, #816) was green incl. CodeQL
                               #   37126532599 while its Pages run failed on rustdoc warnings, now gated.
  loc_gate_verified: true        # all first-party src/ and crates/ files ≤ 500 LOC

  # ── Canonical metrics (update in place with date comment) ────
  product_version: "0.3.8"       # crates.io 0.3.6/0.3.7/0.3.8 all published
  main_head: "83e9a6c"           # 2026-10-05: #852 (quality-gates.sh hands cargo nextest a flag it
                               #   accepts) on top of 77bd5f0 (#846 skill-format fail-open + four
                               #   orphaned fixtures wired), 6ec3cf6 (#848 changelog heading gate) and
                               #   2f26214 (#836 SIGTERM parity). Each value here is read from
                               #   `git log --oneline -1 origin/main`, never from the previous entry.
  tests_count: 1044              # 2026-10-05: 1037 + the 2 SIGINT/SIGTERM exit-status tests for `watch`, then +2
                               #   more for `mcp serve --transport sse` in the same
                               #   tests/cli_shutdown_signal.rs (all 4 pass on the branch; the file registers
                               #   4 tests, re-derived from the harness output, not inferred), then +3 from
                               #   `src/shutdown_tests.rs` (one pending-check per arm, added because the
                               #   `--lib`-only mutation profile scored 0.0000% on #836 head `0107c9c`; the
                               #   lib target went 172 filtered-out -> 175 total, measured not inferred);
                               #   not re-run through scripts/coverage-report.sh inventory, so this is a delta on
                               #   the last measured value, not a fresh measurement.
  skills_count: 33               # 2026-09-07: +pr-roast-triage (find .agents/skills -name SKILL.md | wc -l)
  coverage_lines_percent: 74     # 2026-09-18: cargo +nightly llvm-cov --workspace --lib --tests --branch
  coverage_branches_percent: 64  # same run; unit-only targets measure 68/53, hence --tests matters
  adr_registry_count: 95         # 2026-10-01: check-adr-parity.sh ok (registry=95, disk=94, 0003 N/A)
  adr_disk_count: 94
  integration_test_files: 73     # tests/*.rs (2026-10-04 recount: +cli_shutdown_signal.rs)

  # ── Plans pointers ────────────────────────────────────────────
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_07_20.md"
  plans_archive_roots:
    - "plans/.archive/2026-07-20-historical"
    - "plans/.archive/2026-08-08-historical"
  active_plan_set_compact: true
  ci_matrix_machine_derived: true            # 2026-10-05 (#827, PR #837, main 87fa734): `Test Workspace Crates`
                               #   comes from `scripts/ci-workspace-crate-matrix.sh` reading `cargo metadata`, not a
                               #   hand-written list; the script fails if an excluded crate stops being a member or
                               #   loses its dedicated job. Do not re-queue.
  plan_archive_manifest_valid: true  # 2026-10-05: no longer a claim — scripts/check-archive-manifest.sh (PR #838, main f235874) compares the manifest against disk both directions (131 = 131, RC=0) and validate.sh:234-238 refuses if the gate is missing or not executable
  goap_actions_tracked_as_issues: true  # 2026-10-04: reconciliation found 10 queued actions and 0 open issues — the
                               #   backlog existed only on this filesystem. Each action now carries a
                               #   `github_issue:` key; #824..#833 hold the verified evidence (line numbers,
                               #   grep counts, LOC) so a future session can re-derive rather than re-trust.
                               #   Bidirectional, re-measured 2026-10-05 on this base: 8 queued actions and 8
                               #   `github_issue:` keys — 8 and 8 after the #836 close-out and the #851 filing
                               #   (`grep -c '^  - name:'` vs `grep -c '^    github_issue:'`).
                               #   Open GOAP issues with no queue entry are now an error, not a note:
                               #   #840/#841/#842 are queued in this round, #849 was fixed and merged as
                               #   #852, and #839/#850/#853 stay CI findings that are filed but not yet
                               #   queued (they are not `GOAP:`-titled, so the UNQUEUED check does not
                               #   cover them — if they should be, they need the title prefix).
  fixture_suites_wired_into_validate: true  # 2026-10-05: flipped on measurement, not on the PR — `77bd5f0` (#846) landed the wiring. On main 83e9a6c validate.sh invokes test-check-test-attributes.sh (:25), test-ci-workspace-crate-matrix.sh (:40), test-llms-sync.sh + test-version-sync.sh (:121 loop), test-validate-changelog.sh (:212), test-validate-skill-format.sh (:223) and test-quality-gates.sh (:294). The #829 action no longer claims this key: a queued action advertising an effect reported true here is the #851 drift and `check-goap-queue-issues.sh` fails on it.
  skill_format_gate_fixtured_and_fail_closed: true  # 2026-10-05: also flipped by #846 landing. validate-skill-format.sh:282-287 rejects a `---` opener with no closing delimiter (the fail-open), and the 15-case fixture runs from validate.sh:223 -> CI lint job. Same treatment as the key above: moved out of #829's `effects:`.
  changelog_sections_unique: true  # 2026-10-05 (#832, PR #848, main 6ec3cf6): `## [Unreleased]` holds one heading per release type — `grep -n '^### ' CHANGELOG.md` reads 10 Added / 14 Changed / 33 Fixed — and scripts/validate-changelog.sh scores duplicates per section, fixtured by scripts/test-validate-changelog.sh (10 cases) wired at validate.sh:208-214.
  single_gate_graph: false  # 2026-10-05: the parent claim of #829, kept queued because it is genuinely not done. Measured on 83e9a6c: validate-links.sh (5 broken links, exit 1) and check-docs-links.sh (39 issues, exit 1) are red with no caller; validate-git-hooks.sh had no caller either. Decomposed into #840 / #841 / #842.
  yaml_gate_severity_classified_and_gated: true  # 2026-10-05 (#841, PR #854, main 2984d3e): validate-workflows.sh classifies yamllint output on `-f parsable` into syntax error / error-level / warning-level instead of calling every finding "YAML syntax error", .yamllint gives it a profile (relaxed; line-length 200 warning, priced from ci.yml:516 at 166 cols and benchmark-ci.yml:61 at 207), an unavailable parser prints SKIPPED — NOT A PASS instead of a silent pass, and the 12-case fixture runs from validate.sh:297-303 with yamllint apt-installed in the lint job (pip hits PEP 668 on ubuntu-24.04).

  # ── Active wave ───────────────────────────────────────────────
  active_wave: 33
  wave_32_status: in_progress    # 2026-09-30: exits re-verified — ownership/features/scale-evidence/metres landed; residuals queued (TTL shutdown, absence invalidation, failure-path + query-count tests, gate + catalog work, evidence tiers)
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_33_status: in_progress    # docs truth + missing behavior + evidence; mostly landed
  queued_actions_count: 9        # 2026-10-05 ledger. Every line below is `git show <commit>:plans/ACTIONS.md
                               #   | grep -c '^  - name:'`, not a recollection — and reading the commits
                               #   instead of the prose is what exposed the drift #851 was written about:
                               #   87fa734 (#837 landed) -> 10   entry NOT removed
                               #   f235874 (#838 landed) -> 10   entry NOT removed
                               #   2f26214 (#836 landed) -> 10   entry NOT removed, still `in_progress`
                               #   6ec3cf6 (#848 landed) -> 10   entry NOT removed
                               #   77bd5f0 (#846 landed) -> 8    removed #824/#827/#831, added #851
                               #   83e9a6c (#852 landed) -> 8
                               #   this commit          -> 10   removed #832, queued #840/#841/#842
                               #   this commit          -> 9    #854 merged as 2984d3e during the same
                               #     round and `Fixes #841` closed the issue the freshly written entry
                               #     pointed at — the gate flagged it STALE before the commit was made,
                               #     which is the first time this mechanism caught drift automatically.
                               #   The four earlier "10 -> 9 -> 8 -> 7" lines this entry carried were
                               #   counterfactual: they described the removals that should have happened,
                               #   and git says none of them did. Now machine-checked —
                               #   scripts/check-goap-queue-issues.sh (#851) fails when this number and
                               #   `grep -c '^  - name:' plans/ACTIONS.md` disagree in either direction.
                               #   2026-10-04: #821 completed give_the_sse_transport_an_exit_path
                               #   and #822 completes revive_dead_cli_parity_help_test (11 -> 9), but measuring
                               #   #821's disclosed limitation on the binary (SIGINT exits 0, SIGTERM exits 143
                               #   = default disposition, so a systemd-stopped server still misses the ADR-0099
                               #   stop) queued cover_sigterm_in_server_shutdown (9 -> 10).
                               #   2026-10-03: #817 completed enable_ttl_cleanup_in_long_running_commands
                               #   (9 -> 8), its review queued revive_dead_cli_parity_help_test (8 -> 9), and the
                               #   mutation-test red on this PR's head queued mutation_baseline_the_feature_gated_mcp_module
                               #   (9 -> 10) plus give_the_sse_transport_an_exit_path (10 -> 11) — the second is a
                               #   production defect that the exclusion review exposed, not a coverage wish.
                               #   Counted with `grep -c '^  - name:' plans/ACTIONS.md`, not asserted.

  # ── Open work (flags currently false — the real backlog) ──────
  no_missing_implementations: true            # 2026-08-12: no TODO in src/ crates/
  workspace_implementation_owners_unique: true    # 2026-08-12: csm-cli/csm-wasm dead dupes removed (#626/#627); root owns bodies, crates own binaries/facades
  no_default_features_is_lean: true               # 2026-08-08: no libsql/rayon in no-default tree (ADR-0094)
  msrv_workspace_aligned: true                    # 2026-08-08: all manifests use workspace rust-version 1.88
  persistence_disabled_false_success_removed: true  # 2026-08-11: replace_persistence_disabled_noops (ADR-0094); CLI DB config rejected w/o feature
  wasm_ci_release_artifact_identical: true  # 2026-09-20: CI builds + smoke-tests the release/web package via scripts/build-wasm.sh (same command release.yml publishes); size gate measures that artifact (656 657 B, sha256-pinned)
  performance_claims_have_current_artifacts: true  # 2026-09-17: plans/evidence/scale_2026_09_17 (three manifests, #733/#734)
  critical_skill_evals_passing: false             # behavioral evals deferred
  fuzz_short_runs_on_pr: true                     # fuzz.yml fuzz-short job: 30s runs of changed targets on PRs
  fuzz_scheduled_full_runs: true                  # fuzz-full weekly cron (Sun 03:00 UTC); nightly toolchain + nix shell dropped (PR #690); first scheduled green 2026-09-13 (run 34746887403) after red ×6 (08-02..09-06)
  duckdb_companion_published: true                # 2026-09-27: csm-duckdb 0.3.8 first-published (owner d-o-hub); release.yml lockstep integration queued
  bhvec_xor_simd_measured_slower: true # 2026-10-02: #808 closed as no-impact — main scalar 89.5 ns vs SIMD dispatch 113.3 ns (i5-8350U, AVX2 present, rustc 1.88.0); the dispatch adds memset + memcpy + an out-of-line target_feature call
  benchmarks_prove_performance: true              # 2026-09-21: named runner (plans/REFERENCE_RUNNER.md), canonical criterion baseline (plans/evidence/bench/canonical.json, 88 benches + CI), release-scale evidence (plans/evidence/scale_release_2026_09_21: ANN to 200k, memory/storage to 500k), npm package evidence (plans/evidence/wasm_2026_09_21)
  perf_pr_evidence_gate_enforced: true           # 2026-09-23: ci.yml commitlint job runs scripts/check-perf-pr-evidence.py for `perf(...)` PR titles; PR template carries `## Performance Evidence`
  deferred_namespace_isolation: false             # ADR-0026 multi-tenancy (trigger: user demand)
  deferred_phase2_optimizations: false            # ADR-0024 (trigger: >200k concepts + latency issues)

  # ── Landed invariants worth checking before changes ───────────
  retrieval_implementation_owner_unique: true     # 2026-07-23: csm-retrieval owns contracts
  ann_scale_evidence_current: true                # 2026-09-17: evidence_ann.json (exact/hnsw/lsh/bucket at 1k/10k/50k)
  persistence_contention_evidence_current: true   # 2026-09-17: evidence_persistence.json (error rate 0.905 -> 0.000)
  measured_memory_model_exists: true              # 2026-09-17: 4691 B RSS + 2850 B storage per concept, held-out 0.29 %
  canonical_test_owners_unique: true              # 2026-09-18: facade test copies of owner tests removed; audit in plans/TEST_SURFACE_AUDIT_2026_09_18.md
  coverage_methodology_behavior_based: true       # 2026-09-18: unique compiled behavior + llvm-cov line/branch, not test LOC
  bucketed_recall_scales_with_corpus: true        # 2026-09-21: adaptive multi-probe + budget guard; probe declines to the exact scan instead of truncating
  ten_million_memory_claim_evaluated: true        # 2026-09-17: evaluated NOT supported (43.7 GB RSS / 26.5 GB storage)
  ann_snapshot_revision_validated: true           # ADR-0093: IndexSnapshotEnvelope + ns revision
  ann_config_is_fallible: true                    # validate_index_backend; ADR-0093
  persistence_failure_leaves_memory_unchanged: true # 2026-09-30: durable-first order + reload reconcile (framework_persistence.rs:244-296); failure path untested (queued)
  persistence_implementation_owner_unique: true  # 2026-09-15: root facade re-exports csm-persistence (phases 1-3); no second body
  mcp_full_width_vector_wire_contract: true       # base64 1280-byte HVec + high-bit tests
  no_state_lock_across_io_await: true             # 2026-09-30: durable I/O before singularity locks (framework_persistence.rs:96,177-215), verified by inspection
  benchmark_metrics_mathematically_correct: true  # 2026-09-30: multi-label recall_at_k, log2 NDCG, abstention gold from should_abstain + hand-calculated tests (benchmarks/src/scorer.rs)
  public_f32_apis_validate_input: true            # 2026-08-07: PR #607 prune/neighbors validation
  workspace_ci_matrix_complete: true              # csm-chaos + benchmark tests in CI; crate list hand-maintained (ci.yml:202-241, machine-derived matrix queued)
  cargo_deny_required_in_ci: true
  fuzz_build_required_in_ci: true
  release_wait_event_driven: true                 # 2026-09-30: release.yml triggers on CI completion (workflow_run, head_sha-pinned); no polling ceiling. Proof: runs 36746741481 + 36746882951 green, publishes correctly skipped
  crates_publish_precheck_ownership_aware: true   # 2026-09-30: owners API (free or d-o-hub passes; other owners fail; unknown HTTP fails closed) replaces the published-version comparison; verified live incl. negative control
  csm_duckdb_in_release_publish_order: true       # 2026-09-30: dedicated step after the root publish waits for the root version in the index; companion loop exposes failures instead of swallowing them
  ttl_cleanup_has_bounded_shutdown: true          # 2026-10-01: ADR-0099 — shared CleanupTask, cooperative watch cancel (no abort), bounded await in shutdown(); loop also stops on last-handle drop. 20 TTL + 2 unit tests; mutation-test green. 2026-10-03 (#813, f999998): `shutdown()` now actually runs on the `csm watch` SIGINT path and after the `mcp::serve` transport ends; that call-site wiring rests on review + a manual SIGINT smoke — the `run_watch -> Ok(())` entry-point mutant is excluded in `scripts/mutation_test.sh`. 2026-10-03 (#817): both paths can now actually hold a reaper — see `servers_reap_expired_concepts`
  absence_records_invalidated_on_insert: true     # 2026-10-01: absence rows carry namespace + revision, so any durable mutation (revision bump, ADR-0093) makes them stale; successful probes delete the record. Migration v12; regression + 100% mutation score
  ttl_clamped_on_the_live_builder: true              # 2026-10-02 (#806, 8f9dc3d): the generic builder production actually binds (singularity_types.rs) now clamps to MAX_TTL_SECONDS_LIMIT; superseded by concept_builder_owner_unique below (one clamp site now)
  concept_builder_owner_unique: true                 # 2026-10-03: the `singularity_types::ConceptBuilder` copy is deleted, `csm_memory::singularity::ConceptBuilder` is a `pub use` of the `concept_builder` owner, so one struct and one `MAX_TTL_SECONDS_LIMIT` clamp serve every path. Guarded by two `tests/arch_fitness.rs` checks (single struct definition; five public spellings unify on the owner type). **Breaking (csm-memory)** — corrected by #816's own self-roast: the deleted copy derived `Clone` and the owner does not, so the impl removal is semver-visible to external callers even though zero in-repo call sites existed; `with_metadata` now takes `impl Serialize` with ADR-0012 error capture everywhere.
  servers_reap_expired_concepts: true                # 2026-10-03 (#817): `--ttl-cleanup-interval <SECONDS>` on `csm watch` and `csm mcp serve` (shared `TtlCleanupArgs`, default 0 = reaper off, so no silent deletion). Threading: `create_framework_advanced` gained `ttl_cleanup_interval_seconds` and `create_framework_with_ttl` wraps it; `McpConfig::ttl_cleanup_interval` → `McpHandler::with_ttl_cleanup_interval` → the same funnel. Proven by raw-row counts, not config echoes: the enabled test drives `Singularity::len("_default")` 1 → 0 with no purge call, the disabled tests assert `cleanup.is_none()` and a row that `is_expired()` is still stored. Four public shapes changed (documented as Breaking in the changelog); one-shot commands are untouched.
  rustdoc_warnings_gated: true                       # 2026-10-03 (#817): `scripts/validate.sh` now runs `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, the same invocation `GitHub Pages` runs under `CARGO_BUILD_WARNINGS=deny`. Six `unresolved link to ':model'` warnings from one `src/cli/args.rs` doc line had failed four main pushes (34751139063, 35274354770, 35614812503, 37126532546) with every required check green, so the site had not deployed since 2026-09-09; the doc text is reworded to drop the brackets (`csm query --help` unchanged in shape, llms never carried it) and the gate now fails the PR that would break the deploy.
  mutation_profile_compiles_feature_gated_mcp: true  # 2026-10-03 (#817): the fast mutation profile
                               #   now passes `--features chaotic_semantic_memory/mcp`, and ci.yml pre-warms that build.
                               #   `mcp` is not a default feature and both `chaotic_semantic_memory::mcp` (src/lib.rs:57)
                               #   and `crate::cli::mcp` (src/cli/mod.rs:6) are cfg-gated on it, so head c47d676 scored 25%
                               #   (run 37131481411: total=9 caught=1 missed=3 unviable=5) with `src/cli/mcp.rs:42` MISSED
                               #   twice — mutants of lines the job never compiled. With the feature both flip MISSED ->
                               #   CAUGHT. Build cost measured: CI per-mutant builds 52-54s without the feature, 146-183s
                               #   for a base build that must link rmcp/axum/tower (mutants 145-208s) — over the old 150s
                               #   bound, hence the 0% TIMEOUT local run; bound is now 420s and one-file rebuilds after
                               #   pre-warming measure 39s. `src/cli/commands/inject.rs:18` (run_inject) and `serve` are
                               #   excluded as --lib-unreachable entry points, same class as run_query/run_watch. Residual
                               #   recorded rather than glossed: `src/mcp/**` is still path-excluded so the module has no
                               #   baseline, and the ADR-0099 caller hop (src/mcp/server.rs:98 -> McpHandler::shutdown) is
                               #   UNREACHABLE on the Sse transport, not merely untested — run_sse_server awaits
                               #   axum::serve(listener, app) with no with_graceful_shutdown, and axum 0.7.9's
                               #   Serve::into_future is an infinite accept loop (serve.rs:205-240; tcp_accept returns
                               #   Option, never Err, :474-496), so serve never returns. Stdio does resolve (rmcp
                               #   Waiting::waiting on stdin EOF, src/mcp/server.rs:83-86), and only the callee is covered
                               #   (src/mcp/tools_tests.rs:231); tests/mcp_sse_integration.rs:17 abort()s the task at :112.
  sse_transport_has_exit_path: true                    # 2026-10-04 (#821, dc7cd9a): `serve_with_shutdown(config,
                               #   shutdown_signal)` is the public entry and the SSE arm now runs
                               #   `axum::serve(listener, app).with_graceful_shutdown(..)`, so the transport resolves
                               #   and ADR-0099's `handler.shutdown()` hop executes on both transports. Asserted for
                               #   its effect in `src/mcp/server_tests.rs` (handle slot Some -> signal -> await ->
                               #   None), which is why disabling the hop turns that test red. This flag was left at
                               #   `false` by the previous round while its own `queued_actions_count` comment
                               #   recorded the completion — a duplicate-key-free file can still self-contradict.
  servers_exit_on_sigterm: true                        # 2026-10-05 (#824, PR #836, main 2f26214): `src/shutdown.rs`
                               #   `operator_shutdown()` selects on SIGINT and SIGTERM, and both long-running callers
                               #   go through it (`src/cli/commands/watch.rs:91`, `src/mcp/server.rs:79`), so a service
                               #   manager's stop reaches ADR-0099's bounded reaper shutdown. Measured on the binary
                               #   before the fix: exit 0 on SIGINT, 143 on SIGTERM, `grep -rn "SIGTERM" src/ tests/
                               #   crates/` = 0. Held `false` on 2026-10-05 morning while #836 was still a branch —
                               #   world state tracks the repository, not the queue. Kept in the lib target as well
                               #   (`src/shutdown_tests.rs`) because the `--lib`-only mutation profile scored 0.0000%
                               #   on head `0107c9c` with all three `src/shutdown.rs` mutants surviving.
  skill_validation_fail_closed: true              # wired into validate.sh + CI + pre-commit
  llms_dependency_versions_current: true          # 2026-09-27: llms.txt/llms-full.txt regenerated (otel 0.32, rmcp 3.4); scripts/check-llms-sync.sh drift gate runs in validate.sh (CI lint job)
  retrieval_string_clones_removed: true           # 2026-09-27: #781 borrowed expansion ids/labels + positions scoring; 2209 -> 151 allocs per query (top_k=10)
  retrieval_cache_invalidated_on_mutation: true   # 2026-09-29: insert/upsert and association mutations invalidate cached results; regression-covered
  capacity_upsert_preserves_existing: true        # 2026-09-29: existing IDs are detected before max_concepts eviction; regression-covered
  mutation_ci_enforced: true
  mutation_threshold: 85
  actions_pinned_to_sha: true
  harness_msrv_current: "1.88"
  adr_0095_accepted: true        # 2026-09-17: plans/adr/0095-evidence-driven-quality-gates.md is Accepted (2026-07-16); precondition of deduplicate_test_and_source_surfaces
  dependabot_alerts_open: 3       # 2026-09-17: otel GHSA cleared (#729); left: lru (quinn-proto/tantivy pin ^0.12) + libsql-sqlite3-parser ×2 (no fix, denied in deny.toml)

  # ── GOAP bookkeeping ──────────────────────────────────────────
  goap_reconciliations_complete:
    - adr_0084_2026_05_20
    - adr_0085_2026_06_06
    - adr_0089_2026_06_16
    - adr_0092_2026_07_11
    - adr_0097_2026_08_08
  goap_state_duplicate_key_fixed: true  # benchmark_workspace_tests_run_in_ci dup removed 2026-08-08

  # Must remain the LAST key and appear exactly once (see header).
  action_last_completed: classify_yaml_findings_in_validate_workflows  # 2026-10-05 #841 via PR #854 (main 2984d3e) — queued during this round and removed during it, caught STALE by scripts/check-goap-queue-issues.sh itself; before that deduplicate_unreleased_changelog_headings (#832, PR #848, main 6ec3cf6), entry removed in this round; earlier the same day cover_sigterm_in_server_shutdown (#824, 2f26214), validate_archive_manifest_completeness (#838, f235874) and derive_ci_crate_matrix_from_workspace (#837, 87fa734) all landed and had their entries removed only at 77bd5f0 — see the queued_actions_count ledger for what git measured.
