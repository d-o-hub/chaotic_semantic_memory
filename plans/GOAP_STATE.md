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
  documentation_complete: true       # 2026-10-20: MCP, versioned exports, OTLP, and same-SHA release docs reconciled; link/changelog/llms gates pass
  validated: true                   # 2026-10-22, measured on the tree this commit lands:
                               #   `scripts/validate.sh` VALIDATE_RC=0 — fmt, test-attribute gate, script
                               #   catalog, safety fixtures, crate matrix, hooks, clippy --all-targets
                               #   --all-features -D warnings, cargo deny --locked, rustdoc, the full suite
                               #   (1170 passed / 0 failed / 1 ignored over 101 targets), the 500-LOC gate,
                               #   the wasm32 check, the wasm size gate, wasm-pack, llms sync, ADR registry,
                               #   the GOAP parser and queue gates, workflow YAML, actionlint and RUSTSEC
                               #   parity. `scripts/harness-check.sh all` HARNESS_RC=0 on six sensors
                               #   (fmt, clippy, deny, test, arch fitness, shared conventions). It took
                               #   three earlier reds to get here — the stale script catalog, an ENOSPC in
                               #   lint_cache.sh that was never root-caused, and the two real defects this
                               #   branch fixes: the WASM-only compile error (VALIDATE_RC=101) and main's
                               #   unparsable release.yml.
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
  product_version: "0.3.9"       # 2026-10-22: `main` carries 0.3.9 (prepared by #932's version
                               #   work); crates.io 0.3.6/0.3.7/0.3.8 are published. The wave branch
                               #   took main's version surfaces (VERSION, root + crate manifests)
                               #   rather than its own 0.3.8.
  main_head: "0f77c0d0"          # 2026-10-22: re-created the wave against this head (a fresh branch
                               #   off `origin/main` + one merge), because #932 rewrote `release.yml`
                               #   on the same mechanism the wave touched. Read from
                               #   `git log --oneline origin/main`, never from the previous entry.
  tests_count: 1152              # 2026-10-22: 1149 + 3 (the import-validation guards the gate could not
                               #   see — duplicate concept id, duplicate association edge, and the
                               #   merge-mode live-endpoint allowance — now have lib-target tests, each
                               #   demonstrated red by disabling its own guard). Fresh inventory:
                               #   tests/ 554, src/ 259, crates/*/src 319, crates/*/tests 20.
                               # 2026-10-22 (earlier this session): a fresh inventory, not a delta — the pre-commit /
                               #   `coverage-report.sh` unique-(file,fn) count is 1149 (tests/ 554,
                               #   src/ 256, crates/*/src 319, crates/*/tests 20). The same tree's
                               #   full run reports `1167 passed; 0 failed; 1 ignored` summed over 101
                               #   test targets; the 18-test difference is the same fn registered in
                               #   more than one target, which is why the inventory metric, not the
                               #   run line, is the canonical count here. `docs/architecture/
                               #   context.yaml` total_tests/tests_passing carry this same number
                               #   (tests/validate_llm_context.py compares the three).
                               # 2026-10-21: 1054 + 10 (#926): index_dir's 5 inline tests became
                               #   11 in `index_dir_tests.rs` (extracted, not stripped) plus 4 in
                               #   `index_dir_summary_tests.rs` — 15 registered in the lib-target run,
                               #   all mutation-visible. Delta on the last measured value.
                               # 2026-10-15: 1044 + 8 (#833) + 2 (#897) = 1054. The #833 half:
                               #   the 8 lib-target tests added for the src/mcp
                               #   mutation baseline (#833): 7 in `src/mcp/handler_tests.rs` and 1 in
                               #   `src/mcp/wire_tests.rs`. The file registered 8 tests when the mcp
                               #   filter ran 32 passing (24 pre-existing), and the mutation run
                               #   re-verified each survivor against them. Delta on the last measured
                               #   value, not a fresh full-suite inventory — same convention as the
                               #   entry below.
                               # 2026-10-05: 1037 + the 2 SIGINT/SIGTERM exit-status tests for `watch`, then +2
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
  adr_registry_count: 98         # 2026-10-20: check-adr-parity.sh ok (registry=98, disk=97, 0003 N/A).
                               #   The three new rows are ADR-0100 (versioned Postcard export), ADR-0101
                               #   (MCP Streamable HTTP + security) and ADR-0102 (bounded OTLP lifecycle).
                               #   The gate's id pattern was `00[0-9]{2}`, so ids >= 0100 were invisible to it;
                               #   it now matches all four-digit IDs (`[0-9]{4}`), with missing/present
                               #   fixtures for 0100 and 1000. 97 unique ids on disk vs 98 files: ADR-0024 has two files.
  adr_disk_count: 97
  integration_test_files: 73     # tests/*.rs (2026-10-20 recount: `ls tests/*.rs | wc -l` = 73 — the
                               #   new mcp_streamable_http_integration.rs replaced the deleted
                               #   mcp_sse_integration.rs, so the count is unchanged)

  # ── Plans pointers ────────────────────────────────────────────
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_10_19.md"
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
  single_gate_graph: true  # 2026-10-10 (#829): re-measured on 4a8b4f84 once all three decomposed
                           #   children landed — negative-fixtures.sh and validate-links.sh deleted
                           #   (#846/#869), test-llms-sync.sh/test-version-sync.sh (validate.sh:134),
                           #   validate-workflows.sh (:327) and check-docs-links.sh (:238) wired with
                           #   fixtures, validate-git-hooks.sh (:49) wired with test-hook-bootstrap.sh,
                           #   one installer (install-hooks.sh; setup-hooks.sh + --install delegate,
                           #   scripts/pre-commit.sh a documented shim), and `run: bash
                           #   scripts/validate.sh` (ci.yml:658) executes all of them in the lint job.
  yaml_gate_severity_classified_and_gated: true  # 2026-10-05 (#841, PR #854, main 2984d3e): validate-workflows.sh classifies yamllint output on `-f parsable` into syntax error / error-level / warning-level instead of calling every finding "YAML syntax error", .yamllint gives it a profile (relaxed; line-length 200 warning, priced from ci.yml:516 at 166 cols and benchmark-ci.yml:61 at 207), an unavailable parser prints SKIPPED — NOT A PASS instead of a silent pass, and the 12-case fixture runs from validate.sh:297-303 with yamllint apt-installed in the lint job (pip hits PEP 668 on ubuntu-24.04).

  # ── Active wave ───────────────────────────────────────────────
  release_pipeline_validated: false  # 2026-10-30: v0.3.9 is prepared (Cargo.toml 0.3.9, one dated
                               #   CHANGELOG header, link row) and unpublished — crates.io and both npm
                               #   channels sit at 0.3.8 and no v0.3.9 tag exists. Flip true only when the
                               #   tag, 9 crates, both npm packages and the release assets all verify.
  audit_followup_wave_merged: true   # 2026-10-30: PR #947 squashed into main as 3953e818 (12:55:38Z);
                               #   roasted before merge in plans/PR_ROAST_2026_10_30.md, mutation gate
                               #   measured at 94.0476% on that head.
  release_gate_race_repaired: true   # 2026-10-30: PR #962 replaced release.yml's one-shot pre-release
                               #   gate read with a bounded 30m wait (#953). Not yet exercised — no
                               #   jobful Release run has happened since #947 restored parsability.
  active_wave: 33
  wave_32_status: in_progress    # 2026-09-30: exits re-verified — ownership/features/scale-evidence/metres landed; residuals queued (TTL shutdown, absence invalidation, failure-path + query-count tests, gate + catalog work, evidence tiers)
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_33_status: in_progress    # docs truth + missing behavior + evidence; mostly landed
  queued_actions_count: 12       # 2026-10-30: eight ci/release follow-ups + #961, #964, #965, #966 (gates)
                               #   (#960 #954 #955 #948 #949 #951 #952 #959) plus #961, the one genuine
                               #   defect the #956/#957/#958 drain surfaced. Rule 11 order: issue first,
                               #   then the dependency-ordered action. Gate: `grep -c '^  - name:
                               #   plans/ACTIONS.md` reads 9, which is what the checker compares.
                               # 2026-10-15: removed the completed
                               #   `mutation_baseline_the_feature_gated_mcp_module` (#833) — its effect is
                               #   now recorded as `mcp_module_mutation_baselined` below, with the evidence in
                               #   `plans/evidence/mutation_mcp_2026_10_15/` (60 mutants: 34 missed / 18 caught
                               #   / 8 unviable before; 50/50 killable caught after). The queue is empty:
                               #   `grep -c '^  - name:' plans/ACTIONS.md` = 0, which is what the gate compares.
                               # 2026-10-12: removed closed #828 (#880 merged as d951067a, Fixes #828); 2026-10-10: removed closed #829 (issue closed with the claim-by-claim re-measurement); removed closed #840 (PR #869, 1fd790e8); earlier that day removed closed #830 (3ef32d5d) and #826 (81c292de). 2026-10-09: removed closed #842 (#862 landed the hook consolidation; the gate flagged the stale entry while shipping the record PR). 2026-10-08: removed closed #825, already implemented by #858 on base d611e12. Historical 2026-10-05 ledger: Every line below is `git show <commit>:plans/ACTIONS.md
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
  critical_skill_evals_passing: true              # 2026-10-20: five critical skills x 4 deterministic cases = 20
                               #   cases, each response judged against every case assertion
                               #   (`judge_batch`, typesafe/jev-1.13.0). Final run: 20/20 pass, mean
                               #   probability 0.871, no judge failures. Evidence and the two skill-rule
                               #   changes the first run forced: `plans/evidence/skill_eval_2026_10_20.md`.
                               #   Functional pass rate only — no claim about time saved or run-to-run
                               #   consistency, and the lowest case (0.58) is a deliberate follow-up.
  binary_export_codec_maintained: true            # 2026-10-20 (ADR-0100): new public binary snapshots are
                               #   written with Postcard behind the `CSMBIN01` envelope; bincode 1.x
                               #   (RUSTSEC-2025-0141) remains only for legacy readers and internal ANN
                               #   storage. The writer streams through a size-limited writer, so the
                               #   100 MiB import limit is enforced during serialization.
  import_apply_path_shared_across_targets: true   # 2026-10-22: the wave rewrote `WasmFramework::import_from_bytes`
                               #   to call `apply_import_payload`, which then lived in `#[cfg(not(target_arch = "wasm32"))]
                               #   mod framework_ops_import` — so the WASM lib did not compile and only
                               #   `cargo check --target wasm32-unknown-unknown -p csm-wasm` (validate's last stage)
                               #   could see it: VALIDATE_RC=101 after clippy, deny, doc, the whole suite and the LOC
                               #   gate had all passed. Fixed by extracting the filesystem-free transactional core to
                               #   `src/framework_import_apply.rs` (ungated, 163 LOC; `framework_ops_import.rs` keeps
                               #   the file/backup ops at 205 LOC) so native, byte-API and WASM imports share one
                               #   validate-then-install path instead of a weaker per-target copy. Same build also
                               #   exposed a dead `#[cfg(target_arch = "wasm32")] use js_sys::Date;` in the wave's new
                               #   `framework_sequence.rs`. Rule: `AGENTS.md` Key Invariants (Cross-Target
                               #   Compilation) + `agents-docs/coding-standards.md` §5.
  mcp_transport_is_current_protocol: true         # 2026-10-20 (ADR-0101): `Transport::Sse` -> `Transport::StreamableHttp`
                               #   (breaking, no alias), rmcp's current stateless Streamable HTTP path with
                               #   the session manager kept only for negotiated legacy versions. Non-loopback
                               #   binds require a bearer token; Host allow-list and Origin validation are
                               #   enforced; the token is compared in constant time.
  otlp_export_is_bounded_and_observable: true     # 2026-10-20 (ADR-0102): bounded batch span processor with
                               #   explicit queue/batch/delay/request-timeout/shutdown bounds; the export
                               #   timeout is bound on the gRPC channel because the SDK exposes
                               #   `BatchConfigBuilder::with_max_export_timeout` only behind its experimental
                               #   async-runtime processor. `force_flush`/`shutdown` report failures.
  fuzz_short_runs_on_pr: true                     # fuzz.yml fuzz-short job: 30s runs of changed targets on PRs
  fuzz_scheduled_full_runs: true                  # fuzz-full weekly cron (Sun 03:00 UTC); nightly toolchain + nix shell dropped (PR #690); first scheduled green 2026-09-13 (run 34746887403) after red ×6 (08-02..09-06)
  duckdb_companion_published: true                # 2026-09-27: csm-duckdb 0.3.8 first-published (owner d-o-hub); release.yml lockstep integration queued
  bhvec_xor_simd_measured_slower: true # 2026-10-02: #808 closed as no-impact — main scalar 89.5 ns vs SIMD dispatch 113.3 ns (i5-8350U, AVX2 present, rustc 1.88.0); the dispatch adds memset + memcpy + an out-of-line target_feature call
  bm25_posting_length_sort_measured_slower: true # 2026-10-19: #917 closed as no-impact — sorting `query_weights` by `entries.len()` is a semantic no-op on every bm25 corpus in the repo (all posting lists are equal length: `benches/benchmark.rs:711-719`, `benches/bm25_benchmark.rs:19-29`, `selective_100000`), and on a purpose-built skewed corpus (20 000 docs; postings 20 000/2 000/200; query descending) it measures **+8 %** (median 168.3 → 180.0 µs, consistent sign over four time-interleaved A/B pairs, `csm-ref-01`). Parity held: bit-identical scores and identical top-5 on a tie corpus. A resubmission must beat +8 % against on a committed unequal-length benchmark; see `plans/PR_ROAST_2026_10_19.md`. Do not re-queue.
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
  persistence_failure_leaves_memory_unchanged: true # 2026-10-08: durable-first/reconcile invariant; all three failure tests from #858 pass in hosted CI 37836615208.
  persistence_failure_semantics_tested: true     # #825 completed in #858; src/framework_persistence_tests.rs covers inject/delete/batch with simulated persistence failure.
  persistence_implementation_owner_unique: true  # 2026-09-15: root facade re-exports csm-persistence (phases 1-3); no second body
  mcp_full_width_vector_wire_contract: true       # base64 1280-byte HVec + high-bit tests
  no_state_lock_across_io_await: true             # 2026-09-30: durable I/O before singularity locks (framework_persistence.rs:96,177-215), verified by inspection
  benchmark_metrics_mathematically_correct: true  # 2026-09-30: multi-label recall_at_k, log2 NDCG, abstention gold from should_abstain + hand-calculated tests (benchmarks/src/scorer.rs)
  public_f32_apis_validate_input: true            # 2026-08-07: PR #607 prune/neighbors validation
  workspace_ci_matrix_complete: true              # csm-chaos + benchmark tests in CI; crate list hand-maintained (ci.yml:202-241, machine-derived matrix queued)
  cargo_deny_required_in_ci: true
  fuzz_build_required_in_ci: true
  ruleset_requires_core_ci_gates: true  # 2026-10-09 (#850): ruleset 12897801 requires lint/test/mutation-test (app 15368) in addition to Build CLI/Codacy/commitlint; strict up-to-date and admin PR bypass retained. Verified via the stored ruleset, /rules/branches/main and `gh pr checks 867 --required` (3 -> 6 contexts).
  scheduled_evidence_tier_runs: true  # 2026-10-10 (#830 -> PR #861, 3ef32d5d): scheduled-evidence.yml runs scale-evidence + a full mutation sweep weekly (dispatch included); benchmark-ci path filters cover crates/**.
  mutation_inventory_published: true  # 2026-10-10 (#830 -> PR #861): module-level table appended to progress/mutation/<profile>-latest.md and uploaded as artifacts.
  mcp_module_mutation_baselined: true  # 2026-10-15 (#833): `src/mcp/*` left the profile's exclusion list
                               #   (`--exclude "src/mcp/*"` plus its two `--exclude-re` label entries at
                               #   `scripts/mutation_test.sh:242-244`) after a scoped baseline: 60 mutants over
                               #   handler/schema/server/tools, **34 missed / 18 caught / 8 unviable** before;
                               #   `src/mcp/handler_tests.rs` (7) + `src/mcp/wire_tests.rs` (1, a real rmcp
                               #   server over `tokio::io::duplex` — the only route to `initialize`/`list_tools`/
                               #   `list_resources`) then took **37/37 of the re-verified set to caught**, i.e.
                               #   50/50 killable with the two documented exclusions (`replace serve ->`, entry
                               #   point; `replace | with ^ in parse_hvec_u64_halves`, equivalent mutant).
                               #   `serve_with_shutdown` is killed, not excluded: its test now requires the
                               #   listener to accept a connection before signalling. Evidence:
                               #   `plans/evidence/mutation_mcp_2026_10_15/`.
                               #   Residual, measured: CI's job budget is `timeout-minutes: 45` and the PR path
                               #   is `--in-diff`, so the mutant count follows changed lines; a diff rewriting
                               #   all of `src/mcp/tools.rs` (31 mutants at 86-108 s of rebuild each) would
                               #   pressure that budget. Not a regression — those lines previously produced no
                               #   results at all.
  link_validators_single_and_wired: true  # 2026-10-10 (#840 -> PR #869, 1fd790e8): one link checker (check-docs-links.sh) wired into validate.sh + the lint job with a 9-case fixture; validate-links.sh deleted.
  skill_catalog_generated_and_gated: true  # 2026-10-12 (#828 -> PRs #847/#880): CATALOG.md is generated from the tree and drift-gated (#847); docs/architecture/context.yaml counters are compared against live sources by tests/validate_llm_context.py (skills/ADRs/active_wave/tests/LOC coverage, module and key-file existence), wired into validate.sh with a 12-case fixture (#880, d951067a); gen-agents-context.sh retired.
  release_wait_event_driven: true                 # 2026-10-22, re-measured against `0f77c0d0` (#932):
                               #   `release.yml` is triggered by successful **CI** completion on `main`,
                               #   resolves that run's `head_sha` as its target commit, and refuses to tag
                               #   or publish until the CI run *and* the `pre-release-gate.yml` run for
                               #   that exact SHA are completed/success; a manual dispatch — including
                               #   `recover` with a `recovery_tag` — runs the same verification. The wave's
                               #   own design (subscribe to the gate rather than to CI) was superseded by
                               #   #932's verify-inside-the-job shape, and the difference is priced here:
                               #   because the gate starts only after CI, a gate still in flight when CI
                               #   finished makes the release **refuse**, not wait, so the operator
                               #   re-dispatches. `dist.yml` is dispatch-only (its `disabled-notice` job
                               #   stands in for a tag trigger that cannot fire for a tag created under
                               #   `GITHUB_TOKEN`). A main push with no PR diff is explicitly “mutation
                               #   not run,” not a passing score. Decision in ADR-0042; operator contract
                               #   in `.agents/skills/release-management/SKILL.md`.
                               #   2026-10-22 AMENDED — and this is the publication blocker: the file as
                               #   landed in `0f77c0d0` (#932) could not be parsed at all. `if:> -` compared
                               #   against double-quoted `”workflow_dispatch”` / `”push”` / `”success”`
                               #   (GitHub expressions accept only single-quoted string literals) and a step
                               #   compared `matrix.target == aarch64-unknown-linux-gnu` unquoted. Every run
                               #   since has produced **zero jobs**: 39670046551, 39678113001, 39683503746 and
                               #   39734351757 (on `0f77c0d0` itself) are all `conclusion: failure` with an
                               #   empty job list, i.e. rejected at workflow parse, not a failed step. The
                               #   last successful release run is 39648046550 on `4a85fd47`, before #932.
                               #   Main's own gate did not notice because the actionlint stage that caught it
                               #   is this wave's (c85984de); this branch quotes all four literals.
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
  workspace_default_members_cover_all: true   # 2026-10-17 (#910, main f496d319): bare
                               #   `cargo clippy/test/doc` at the root now select all 12 members, and
                               #   `scripts/ci-workspace-crate-matrix.sh` enforcement 5 fails when a
                               #   member is missing from `default-members`. Fallout repaired in the same
                               #   landing: the two `csm` bins share a rustdoc output path (root bin
                               #   `doc = false`), `--bin csm` matches two packages (every such command is
                               #   `-p`-scoped), five member intra-doc links, and the bare wasm checks.
                               #   The hooks keep a `--exclude csm-duckdb` bounded subset — its bundled
                               #   DuckDB C++ build is ~20 min cold and blows pre-push's 180 s budget.
  pre_release_gate_no_verdict_for_skipped_runs: true  # 2026-10-17 (#903): non-success CI is no verdict
  audit_ignore_sets_gated_in_parity: true             # 2026-10-17 (#903): checker + fixture in validate.sh
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
  action_last_completed: repair_the_release_gate_race  # 2026-10-30 — PR #962 squashed
                               #   `release.yml`'s one-shot `Verify CI and Pre-Release Gate` read with a
                               #   bounded poll. `release.yml` and `pre-release-gate.yml` both trigger on
                               #   the same CI completion, so `validate` (62 s measured on run 39648046550)
                               #   always read the gate before its verdict existed (slowest measured pass
                               #   4m37s, run 39060608423). #953 is a code-read prediction, not a recorded
                               #   refusal: the workflow was unparsable from 0f77c0d0 until #947, so no run
                               #   ever reached the step — the PR body and the code comment both say so.
                               #   Gates: check-actionlint.sh rc 0, validate-workflows.sh --check rc 0
                               #   (10 files, 0 errors), commitlint rc 0. Previous action:
                               #   `repair_the_merged_tree` (2026-10-22), whose evidence stays in
                               #   `plans/PR_ROAST_2026_10_21.md` and in the git history of #947.
                               #   into the audit-follow-up wave produces a tree neither parent tested.
                               #   The name drops `and_land_the_wave`: the wave is committed locally on
                               #   a branch re-created off `0f77c0d0`, and no push, PR or merge has
                               #   happened (operator scope for this session is validate + commit only),
                               #   so landing is an unfinished action, not a completed one. The gate then
                               #   caught an eighth cause the native suite cannot see: the WASM import
                               #   binding called into a `not(wasm32)` module — see
                               #   `import_apply_path_shared_across_targets`.
                               #   The full suite (not `--lib`) is the only honest check of a merged
                               #   head; it reported eight failures in seven targets, each root-caused
                               #   before being fixed: the consolidated text probe had dropped main's
                               #   absence persist/clear and its record key was not mode-qualified
                               #   (`fix(retrieval)`), `OtlpBatchConfig::validate()` left
                               #   `shutdown_timeout` unbounded, the MCP wire handshake asserted an
                               #   impossible protocol echo and the stateless Streamable HTTP requests
                               #   omitted the SEP-2243 `Mcp-Method` header the `2026-07-28` revision
                               #   makes mandatory (`-32020` → 400), eight tests wrote scratch to a
                               #   quota-capped `/tmp` or to a `TMPDIR` the path guard rejects, one CLI
                               #   test invoked a `--format` flag `query` never had, and three of main's
                               #   tests pinned import/backup contracts this wave replaces on purpose —
                               #   the lib target already pinned the new ones, which is why a `--lib`
                               #   view looked clean. `Cargo.lock` is spliced from `main`'s lock plus the
                               #   four stanzas the manifests newly require (re-resolving reproduces the
                               #   21 unrelated version-reference flips), leaving 35 insertions and 0
                               #   deletions. Previous action: `implement_audit_followups_2026_10_20`.
