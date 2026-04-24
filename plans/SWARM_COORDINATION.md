# Swarm Coordination

## Combined Agents (Generated from Skills)

| Agent | Skills Combined | Use For |
|-------|-----------------|---------|
| @impl | rust-development + testing-validation | Implementation |
| @fix | rust-development + testing-validation + debugging-reservoir | Bug fixes |
| @perf | benchmarking-perf + debugging-reservoir + swarm-performance | Performance |
| @test | testing-validation + swarm-testing-quality | Testing |
| @plan | goap-planning + adr-creation | Planning/ADR |
| @ci | github-ci-guardrails + git-workflow | CI/CD |
| @swarm | all swarm skills | Full swarm |

Generated via: `scripts/generate-agents.sh`

## Active Swarm Groups

| Group | Phase | Focus | Status |
|-------|-------|-------|--------|
| A | 48 | Probe Scan Path | Complete |
| B | 48 | SQLite WAL Policy | Complete |
| C | 48 | Validation Coverage | Complete |
| D | 48 | GOAP/ADR/Docs Sync | Complete |

## Swarm Status: **ACTIVE** 🟢

Waves 1-20 complete. Wave 21 dependency analysis documented.

## Orchestrator Run (2026-04-24, Wave 21)

Run ID: `goap_wave21_impl_queue_resolution_2026_04_24`

### Resolved IMPLEMENTATION_QUEUE (Wave 20 -> 21)

| Queue ID | Task | Final State | Notes |
|----------|------|-------------|-------|
| IQ-01 | `upgrade_libsql_0_4_to_0_9` | ✅ complete | Already at 0.9.30 |
| IQ-02 | `upgrade_rand_0_8_to_0_9` | ✅ complete | Already at 0.10.1 |
| IQ-03 | `update_getrandom_wasm_flag` | ✅ complete | wasm_js feature enabled |
| IQ-04 | `evaluate_bincode_to_postcard` | ⏳ deferred | v0.4.0 (backward compat) |
| IQ-05..IQ-10 | skill-memory hardening | ✅ verified | Scripts already complete |
| IQ-11 | `configure_npm_oidc_trusted_publisher` | 🔒 blocked | External account required |
| IQ-12 | `automate_npm_publish_flow` | 🔒 blocked | Depends on IQ-11 |
| IQ-13 | `add_error_source_attributes` | ✅ complete | #[source] on all variants |
| IQ-14 | `enhance_error_context_hints` | ✅ complete | Remediation hints added |
| IQ-15 | `add_property_security_tests` | ✅ complete | 13 tests in security_proptest.rs |
| IQ-16 | `add_avx2_neon_simd_paths` | ✅ complete | hyperdim_simd.rs (109 LOC) |

### Wave 21 Handoff Artifact

- `plans/handoffs/W21_dependency_upgrade_analysis.md`

### Wave 20 Handoff Artifacts (resolved)

| Group | Focus | Agent | Max Parallel | Queue IDs |
|-------|-------|-------|--------------|-----------|
| A | Implementation (feature work) | `@impl` | 2 | IQ-05, IQ-06, IQ-07, IQ-08, IQ-09, IQ-10 |
| B | Fix/Security hardening | `@fix` | 2 | IQ-13, IQ-14 |
| C | Performance | `@perf` | 1 | IQ-16, IQ-04 |
| D | Testing | `@test` | 2 | IQ-15 + regression suites for IQ-01..IQ-10 |
| E | Planning/ADR | `@plan` | 1 | IQ-01, IQ-02, IQ-03 (breaking changes and sequencing) |
| F | CI/Release/Ops | `@ci` | 1 | IQ-11, IQ-12 |

### Dependency and Handoff Chain

1. `E -> A/B/C`: dependency/ADR decisions for upgrades and migration boundaries.
2. `A -> D`: implementation contracts, retry/health/metrics/log-rotation APIs, and edge cases.
3. `B -> D`: error taxonomy and source-chain expectations.
4. `C -> D`: SIMD/serialization perf assumptions and benchmark acceptance thresholds.
5. `F -> All`: npm publisher prerequisites and CI gating strategy.
6. `D -> F`: final validation evidence before CI release gate.

### Wave 20 Handoff Artifacts

- `plans/handoffs/W20_A_to_D_impl_contract.md`
- `plans/handoffs/W20_B_to_D_fix_contract.md`
- `plans/handoffs/W20_C_to_D_perf_contract.md`
- `plans/handoffs/W20_E_to_All_planning_contract.md`
- `plans/handoffs/W20_F_to_All_ci_blockers.md`

### Current State
- **Phase 48 complete** (probe-path allocation removal + local SQLite WAL policy)
- **Validation gates passing** (`scripts/validate.sh` + LOC + WASM size gate)
- **New handoff artifact:** `plans/handoffs/W17_Phase48_Performance_Followup.md`
- **ADR-0056 implemented** with deferred ANN/LSH trigger preserved

### Wave 17 (completed) - Performance Follow-up & GOAP Consistency (Phase 48)

Run ID: `goap_wave17_phase48_2026_03_09`

| Action | Priority | Status | ADR |
|--------|----------|--------|-----|
| `remove_probe_scan_materialization` | P1 | Complete | ADR-0056 |
| `enable_local_sqlite_wal` | P1 | Complete | ADR-0056 |
| `add_wal_validation_coverage` | P2 | Complete | ADR-0056 |
| `sync_goap_and_docs_state` | P2 | Complete | ADR-0056 |

**Status:** Wave 17 COMPLETE ✅

## Orchestrator Run (2026-02-17, Wave 7)

Run ID: `goap_swarm_wave7_phase13_16_closure_2026_02_17`

### Parallel Wave 7 (completed)

| Group | Action | Status | Output/Handoff Artifact |
|-------|--------|--------|--------------------------|
| A | `add_to_hypervector_benchmark` + `add_critical_error_path_tests` | Complete | `plans/handoffs/W7_A_to_All_testing_pragmatism.md` |
| B | `document_framework_config` + `document_singularity_config` + `expand_readme_documentation` + `create_basic_in_memory_example` + `add_cargo_aliases` | Complete | `plans/handoffs/W7_B_to_All_documentation_dx.md` |
| C | `add_singularity_tracing` + `add_cache_metrics` + `add_reservoir_metrics` | Complete | `plans/handoffs/W7_C_to_All_observability_completion.md` |
| D | `expose_process_sequence_to_wasm` + `add_wasm_memory_export_import` | Complete | `plans/handoffs/W7_D_to_All_wasm_parity.md` |

### Wave 7 Handoff Contract

1. `A -> All`: critical-path tests and benchmarks close phase 15 quality gates.
2. `B -> All`: config docs, README expansion, and aliases become canonical developer UX entry points.
3. `C -> All`: tracing and metric fields are standardized for framework + wasm snapshots.
4. `D -> All`: wasm temporal + memory parity methods and type declarations are now available.

## Orchestrator Run (2026-02-17)

Run ID: `goap_parallel_missing_tasks_wave5_2026_02_17`

### Parallel Wave 1 (launched)

| Group | Action | Status | Output/Handoff Artifact |
|-------|--------|--------|--------------------------|
| A | `create_fuzzing_targets` | Complete | `fuzz/` corpus + crash triage notes |
| B | `implement_simd_hypervector_ops` | Complete | benchmark deltas + invariant checklist |
| C | `add_structured_logging` | Complete | tracing field conventions + span map |
| D | `add_schema_migration_support` | Complete | migration plan + schema version map |

### Handoff Contract

1. `A -> B`: Provide malformed-input edge cases discovered by fuzzing before SIMD implementation is finalized.
2. `B -> D`: Provide data-layout compatibility notes for batch ops/cache impacts before versioning and export/import work.
3. `C -> A/B/D`: Publish tracing field names and error-context standards before merging each group’s APIs.
4. `D -> All`: Publish schema version increments and migration constraints before integration tests run.

### Phase Boundary Gates

1. After Phase 5 outputs: run full validation and update `GOAP_STATE.md`.
2. After Phase 6 outputs: rerun perf gate and update benchmark deltas.
3. After Phase 7 outputs: verify observability footprint and error context coverage.
4. Before Phase 8 merge: require migration compatibility check + restore/import validation.
5. Before Wave 5 completion: require all performance targets to pass and update `benchmarks_prove_performance`.

### Parallel Wave 2 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `expand_edge_case_coverage` | complete | W1 A |
| B | `add_connection_pooling` | complete | W1 B, W1 A handoff |
| C | `improve_error_context` | complete | W1 C |
| D | `implement_export_import` | complete | W1 D, W1 B handoff |

### Parallel Wave 3 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `enable_mutation_testing` | complete | W2 A |
| B | `add_framework_batch_operations` | complete | W2 B |
| C | `add_metrics_collection` | complete | W2 C |
| D | `add_concept_versioning` | complete | W2 D |

### Parallel Wave 4 (queued)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| B | `implement_concept_lru_cache` | complete | W3 B |
| C | `create_derive_macros` | complete | W3 C |
| D | `implement_backup_restore` | complete | W3 D |

### Parallel Wave 5 (completed)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `benchmark_turso_roundtrip` | complete | W4 complete |
| B | `validate_memory_footprint_10m` | complete | W4 complete, W5 A handoff |
| C | `validate_wasm_binary_size` | complete | W4 complete |
| D | `enforce_performance_goal_gate` | complete | W5 A, W5 B, W5 C handoffs |

### Wave 5 Handoff Contract

1. `A -> B`: Provide Turso latency profile and query mix assumptions for footprint workload alignment.
2. `B -> D`: Provide memory-accounting method and pass/fail evidence for 10M-under-12MB target.
3. `C -> D`: Provide wasm artifact path, measured size, and deterministic CI command.
4. `D -> All`: Publish final performance-goal gate decision with remediation if a target fails.

### Parallel Wave 6 (completed)

| Group | Action | Start Condition | Depends On |
|-------|--------|-----------------|------------|
| A | `finalize_testing_documentation` | complete | W5 complete |
| B | `finalize_performance_benchmarks` | complete | W5 complete |
| C | `finalize_observability_integration` | complete | W5 complete |
| D | `finalize_advanced_features_validation` | complete | W5 complete |

### Wave 6 Closure Contract

1. `A -> All`: Consolidate testing artifacts and coverage reports.
2. `B -> All`: Finalize benchmark baselines and performance regression suite.
3. `C -> All`: Publish observability conventions and tracing span taxonomy.
4. `D -> All`: Validate all advanced features (export/import, versioning, backup/restore) integration.

### Wave 6 Artifacts

- `plans/handoffs/W6_A_to_All_testing_closure.md`
- `plans/handoffs/W6_B_to_All_performance_closure.md`
- `plans/handoffs/W6_C_to_All_observability_closure.md`
- `plans/handoffs/W6_D_to_All_features_closure.md`

### Wave 5 Artifacts

- `plans/handoffs/W5_A_to_B_turso_latency_profile.md`
- `plans/handoffs/W5_B_to_D_memory_budget_report.md`
- `plans/handoffs/W5_C_to_D_wasm_size_report.md`
- `plans/handoffs/W5_D_to_All_performance_gate_decision.md`

### Wave 9 (completed) - CLI Crate

| Group | Action | Status | Output/Handoff Artifact |
|-------|--------|--------|--------------------------|
| A | CLI crate scaffold | Complete | src/cli/ module structure |
| B | CLI commands implementation | Complete | inject, probe, associate, export, import |
| C | CLI tests and completions | Complete | tests/cli_integration.rs |
| D | Shell completions | Complete | `csm completions <shell>` |

**Wave 9 Artifacts:**
- `plans/handoffs/W9_CLI_design.md`
- `plans/handoffs/W9_CLI_edge_cases.md`

### Wave 10 (completed) - Production Hardening (Phases 21-24)

| Phase | Focus | Key Deliverables | Status |
|-------|-------|------------------|--------|
| 21 | Production Hardening | Async lock safety, CLI robustness, WASM panic safety | Complete |
| 22 | CI/DX Hardening | LOC gate recursive, pre-commit hooks, clippy parity | Complete |
| 23 | Rust Best Practices | #[must_use], unsafe docs, CLI JSON serde | Complete |
| 24 | Cargo.toml Modernization | Edition 2024, crates.io metadata, dependency gating | Complete |

**Wave 10 Artifacts:**
- `plans/handoffs/W10_CI_FIX_FINAL.md`
- ADRs: 0032, 0033, 0034, 0035, 0036, 0037, 0038, 0040

### Wave 11 (completed) - Release Engineering (Phase 25)

| Deliverable | Status | Details |
|-------------|--------|---------|
| Release Management Skill | ✅ Complete | `.agents/skills/release-management/` |
| GitHub Pages Workflow | ✅ Complete | `.github/workflows/pages.yml` |
| crates.io Trusted Publishing | ✅ Complete | OIDC-based, no long-lived secrets |
| npm Provenance Publishing | ✅ Complete | WASM bindings with provenance |
| mdBook Documentation | ✅ Complete | GitHub Pages structure |

**Wave 11 Artifacts:**
- `plans/handoffs/W11_Release_Engineering_Complete.md`
- ADR-0039: Release Engineering

## Coordination Rules

1. **Independent Operation**: Groups work on different phases without blocking
2. **ADR Gate**: Any architecture change requires ADR review before implementation
3. **Integration Points**: Phase boundaries require cross-group validation
4. **Conflict Resolution**: First-come-first-served on shared files, coordinate via GOAP_STATE
5. **Configurability Rule**: No hardcoded runtime settings or magic numbers; use named constants and env/config tunables.

## Work Distribution

### Group A: Testing & Quality
- Property-based testing (`proptest`)
- Fuzzing targets (`cargo-fuzz`)
- Edge case coverage

### Group B: Performance
- SIMD hypervector operations
- Connection pooling for Turso
- Framework batch APIs
- LRU concept cache

### Group C: Observability
- Structured logging (`tracing`)
- Metrics collection
- ~~Derive macros~~ (removed - unused)
- Error context enhancement

### Group D: Advanced Features
- Export/import (JSON + binary)
- Concept versioning
- Schema migrations
- Backup/restore operations

## Communication Protocol

1. Before starting: Read `GOAP_STATE.md` to check current status
2. During work: Update `GOAP_STATE.md` with `in_progress` flags
3. On completion: Mark actions complete, update LOC counts
4. On conflict: Document in `SWARM_ISSUES.md` (create if needed)
