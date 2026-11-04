# GOAP World State — canonical current state

> **Compacted 2026-11-03 (ADR-0097 pattern).** This file holds **current truth
> only**: flags an agent needs to plan the next action. The verbatim
> pre-compaction state — including every dated per-wave/PR narrative — is
> archived at
> `plans/.archive/2026-11-03-historical/GOAP_STATE_2026_11_03.md`. Earlier
> snapshots: `plans/.archive/2026-08-08-historical/` and
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
  documentation_complete: true       # 2026-10-20: MCP, exports, OTLP and same-SHA release docs reconciled
  validated: true                    # 2026-10-22: scripts/validate.sh rc 0 + harness-check.sh all rc 0
  ci_all_checks_passed: true         # 2026-10-03: landing head and its push both green (incl. CodeQL/Pages)
  loc_gate_verified: true            # all first-party src/ and crates/ files <= 500 LOC

  # ── Canonical metrics (update in place with a date comment) ──
  product_version: "0.3.9"           # main carries 0.3.9; 0.3.6/0.3.7/0.3.8 are on crates.io
  main_head: "fdc40ef4"              # 2026-11-04: `git rev-parse --short=8 origin/main`
  tests_count: 1188                  # 2026-11-03: `scripts/coverage-report.sh` inventory (tests/ 554,
                                     #   src/ 284, crates/*/src 330, crates/*/tests 20); the full suite
                                     #   reports more passes because a fn can register in several targets
  skills_count: 33                   # find .agents/skills -name SKILL.md | wc -l
  coverage_lines_percent: 74         # 2026-09-18: llvm-cov --workspace --lib --tests --branch
  coverage_branches_percent: 64      # same run; --tests matters (unit-only targets measure 68/53)
  adr_registry_count: 99             # check-adr-parity.sh: registry rows (ADR-0024 has two files)
  adr_disk_count: 98                 # unique ids on disk (99 files, ADR-0024 has two)
  integration_test_files: 73         # ls tests/*.rs | wc -l

  # ── Plans pointers ────────────────────────────────────────────
  plans_active_index: "plans/README.md"
  plans_recommendations_canonical: "plans/RECOMMENDATIONS_2026_10_19.md"
  plans_archive_roots:
    - "plans/.archive/2026-07-20-historical"
    - "plans/.archive/2026-08-08-historical"
    - "plans/.archive/2026-11-03-historical"
  active_plan_set_compact: true
  ci_matrix_machine_derived: true            # 2026-10-05 (#827): derived from `cargo metadata`
  plan_archive_manifest_valid: true          # bidirectional disk check; 172 rows after this compaction
  goap_actions_tracked_as_issues: true       # every queued action names an open `github_issue:`
  fixture_suites_wired_into_validate: true   # every text gate runs its fixture from validate.sh
  skill_format_gate_fixtured_and_fail_closed: true  # ADR-0096 validator + 15-case fixture
  changelog_sections_unique: true            # one heading per release type per version section
  single_gate_graph: true                    # one hook installer; validate.sh runs every gate + fixture
  yaml_gate_severity_classified_and_gated: true  # syntax/error/warning; no-parser prints NOT A PASS

  # ── Active wave / release surface ─────────────────────────────
  release_pipeline_validated: false  # v0.3.9: tag + 2/2 npm + 7 assets live, crates.io 0/9 (dead
                                     #   credential, #967). Stays false until 9/9; recover with
                                     #   `recover=true recovery_tag=v0.3.9`. A green run whose jobs all
                                     #   skipped is not evidence (#969).
  audit_followup_wave_merged: true   # 2026-10-30: PR #947 -> 3953e818
  release_gate_race_repaired: true   # 2026-10-30: bounded 30m same-SHA gate wait (#953/#962)
  release_refusal_names_the_failing_gate_job: true  # #960: gate-failure-detail.sh names the job
  verify_release_collects_all_verdicts: true        # #954/#973: every probe's verdict is collected
  crates_io_index_checks_match_the_index: true      # #971/#973: `"vers":"$VERSION"` matches sparse index
  verify_release_retries_re_query: true             # #972/#973: probes re-query, not replay a substitution
  host_subprocesses_isolated_from_nix_env: true     # #949/#981: 37-case fixture; nix jobs use use-flakehub: false
  active_wave: 33
  wave_32_status: in_progress        # exits re-verified; residuals are queued as individual actions
  wave_32_roadmap: "plans/GOAP_AUDIT_2026_07_14.md"
  wave_33_status: in_progress        # docs truth + missing behavior + evidence; mostly landed
  queued_actions_count: 4            # `grep -c '^  - name:' plans/ACTIONS.md` (#951 #952 #959 #1002)

  # ── Open work (flags currently false — the real backlog) ──────
  deferred_namespace_isolation: false             # ADR-0026 multi-tenancy (trigger: user demand)
  deferred_phase2_optimizations: false            # ADR-0024 (trigger: >200k concepts + latency issues)
  github_action_pins_validated_in_ci: false       # #1002: validate-github-actions-shas.sh is opt-in and
                                                  #   no workflow sets CSM_VALIDATE_GITHUB_ACTIONS_SHAS,
                                                  #   so #1000's nonexistent download-artifact pin reached a runner

  # ── Landed invariants that constrain future changes ───────────
  no_missing_implementations: true            # no TODO/unimplemented! in src/ crates/
  workspace_implementation_owners_unique: true  # root owns bodies; crates own binaries/facades
  no_default_features_is_lean: true             # no libsql/rayon in the no-default tree (ADR-0094)
  msrv_workspace_aligned: true                  # workspace rust-version 1.88 single-sourced
  persistence_disabled_false_success_removed: true  # disabled persistence rejects, never no-ops
  wasm_ci_release_artifact_identical: true      # CI and release both build release/web via build-wasm.sh
  performance_claims_have_current_artifacts: true  # plans/evidence/scale_2026_09_17
  critical_skill_evals_passing: true            # 2026-10-20: 5 skills x 4 cases = 20/20
  binary_export_codec_maintained: true          # ADR-0100: Postcard writer, legacy bincode reader
  import_apply_path_shared_across_targets: true # one validate-then-install path for native/bytes/WASM
  mcp_transport_is_current_protocol: true        # ADR-0101: Streamable HTTP; Host/Origin/token policy
  otlp_export_is_bounded_and_observable: true    # ADR-0102: bounded batch processor; flush reports failure
  fuzz_short_runs_on_pr: true                    # fuzz.yml fuzz-short: 30s on changed targets
  fuzz_scheduled_full_runs: true                 # weekly cron Sun 03:00 UTC; first green 2026-09-13
  duckdb_companion_published: true               # csm-duckdb 0.3.8 first-published (owner d-o-hub)
  bhvec_xor_simd_measured_slower: true           # #808 closed no-impact: 113.3 ns vs 89.5 ns scalar
  bm25_posting_length_sort_measured_slower: true # #917 closed no-impact: +8% on a skewed corpus
  benchmarks_prove_performance: true             # canonical criterion baseline + scale/npm evidence
  perf_pr_evidence_gate_enforced: true           # `perf(...)` titles need `## Performance Evidence`
  squash_subject_length_checked_before_merge: true  # #965/#980: title + " (#N)" within 100 chars
  dependency_bumps_checked_against_pinned_msrv: true # #988: uuid >= 1.27 needs MSRV 1.89; bumps checked
  retrieval_implementation_owner_unique: true     # csm-retrieval owns contracts
  ann_scale_evidence_current: true                # exact/hnsw/lsh/bucket at 1k/10k/50k
  persistence_contention_evidence_current: true   # error rate 0.905 -> 0.000 after busy timeout/retries
  measured_memory_model_exists: true              # 4691 B RSS + 2850 B storage per concept (held-out 0.29%)
  canonical_test_owners_unique: true              # no facade copies of owner tests
  coverage_methodology_behavior_based: true       # unique compiled behavior + llvm-cov, not test LOC
  bucketed_recall_scales_with_corpus: true        # adaptive probe; declines to exact scan, never truncates
  ten_million_memory_claim_evaluated: true        # NOT supported: 43.7 GB RSS / 26.5 GB storage projected
  ann_snapshot_revision_validated: true           # ADR-0093: IndexSnapshotEnvelope + ns revision
  ann_config_is_fallible: true                    # validate_index_backend
  persistence_failure_leaves_memory_unchanged: true # durable-first/reconcile invariant
  persistence_failure_semantics_tested: true      # inject/delete/batch failure tests (src/framework_persistence_tests.rs)
  persistence_implementation_owner_unique: true   # root re-exports csm-persistence; no second body
  mcp_full_width_vector_wire_contract: true       # base64 1280-byte HVec + high-bit tests
  no_state_lock_across_io_await: true             # durable I/O before singularity locks
  benchmark_metrics_mathematically_correct: true  # multi-label recall/NDCG/abstention with hand checks
  public_f32_apis_validate_input: true            # public f32 entry points reject NaN/inf/out-of-range
  workspace_ci_matrix_complete: true              # csm-chaos + benchmarks covered; matrix machine-derived
  cargo_deny_required_in_ci: true
  fuzz_build_required_in_ci: true
  ruleset_requires_core_ci_gates: true            # ruleset 12897801: lint/test/mutation-test + 3 more
  scheduled_evidence_tier_runs: true              # weekly scale evidence + full mutation sweep
  mutation_inventory_published: true              # module-level table + artifacts
  mcp_module_mutation_baselined: true             # #833: src/mcp/* out of the exclusion list
  link_validators_single_and_wired: true          # one checker + 9-case fixture (#840/#869)
  skill_catalog_generated_and_gated: true         # generated CATALOG.md + context.yaml drift gate
  release_wait_event_driven: true                 # ADR-0042: CI completion -> same-SHA gate -> publish
  crates_publish_precheck_ownership_aware: true   # owners API; unknown HTTP fails closed
  csm_duckdb_in_release_publish_order: true       # dedicated step after root; failures exposed
  ttl_cleanup_has_bounded_shutdown: true          # ADR-0099: cooperative cancel + bounded await
  absence_records_invalidated_on_insert: true     # namespace + revision scoped; migration v12
  ttl_clamped_on_the_live_builder: true           # production builder clamps to MAX_TTL_SECONDS_LIMIT
  concept_builder_owner_unique: true              # one struct; five public spellings resolve to it
  servers_reap_expired_concepts: true             # --ttl-cleanup-interval; default 0 = reaper off
  rustdoc_warnings_gated: true                    # RUSTDOCFLAGS=-D warnings cargo doc in validate.sh
  mutation_profile_compiles_feature_gated_mcp: true  # fast profile passes the mcp feature
  sse_transport_has_exit_path: true               # #821: serve_with_shutdown + graceful shutdown
  servers_exit_on_sigterm: true                   # #824: SIGINT and SIGTERM share operator_shutdown()
  skill_validation_fail_closed: true              # validator + fixture in validate.sh + CI + pre-commit
  llms_dependency_versions_current: true          # llms.txt regenerated + drift-gated
  retrieval_string_clones_removed: true           # #781: borrowed expansion ids/labels (2209 -> 151 allocs)
  retrieval_cache_invalidated_on_mutation: true   # insert/upsert/association mutations clear cached results
  capacity_upsert_preserves_existing: true        # existing ids survive max_concepts eviction
  mutation_ci_enforced: true
  workspace_default_members_cover_all: true       # #910: bare root cargo commands select all 12 members
  pre_release_gate_no_verdict_for_skipped_runs: true  # #903: non-success CI is no verdict, not a failure
  audit_ignore_sets_gated_in_parity: true         # deny.toml and .cargo/audit.toml ID sets compared
  mutation_threshold: 85
  actions_pinned_to_sha: true
  harness_msrv_current: "1.88"
  adr_0095_accepted: true                         # plans/adr/0095-evidence-driven-quality-gates.md
  dependabot_alerts_open: 3                       # 2026-11-03 measured: lru + libsql-sqlite3-parser x2
  npm_attestations_verified_and_retry_dropped: true  # 2026-11-03 (#955): both packages' provenance is
                                                  #   read from the abbreviated packument; no retry publish
  chaos_input_dim_rejected_not_clamped: true      # 2026-11-03 (#961): MAX_INPUT_DIM + fallible try_new
  zip_bundled_assumption_gated: true              # 2026-11-03 (#966): stale zip ignores removed; gate ties any
                                                  #   future ignore to zip-in-lock + csm-duckdb/bundled

  # ── GOAP bookkeeping ─────────────────────────────────────────
  goap_reconciliations_complete:
    - adr_0084_2026_05_20
    - adr_0085_2026_06_06
    - adr_0089_2026_06_16
    - adr_0092_2026_07_11
    - adr_0097_2026_08_08
    - adr_0103_2026_11_03
  goap_state_duplicate_key_fixed: true

  # Must remain the LAST key and appear exactly once (see header).
  action_last_completed: bound_chaotic_lsh_and_voronoi_lsh_input_dim  # 2026-11-04 (#961):
                               #   one shared MAX_INPUT_DIM (4096) bounds both projectors; try_new
                               #   rejects zero/oversized dimensions with InputDimensionError, new
                               #   clamps and exposes input_dim(). Previous action:
                               #   verify_npm_attestations_in_the_release_report — 2026-11-03 (#955):
                               #   verify-release checks .dist.attestations via the abbreviated
                               #   packument and the fail-open npm retry is gone.
