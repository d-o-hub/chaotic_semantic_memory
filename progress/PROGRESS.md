# PROGRESS

## 2026-10-02 (feedback round): open-PR blockers addressed

With the trunk green after the WASM freshness repair, the two feedback-bearing PRs in the open queue were roasted and their blockers fixed (the other two open PRs — #806, #807 — carry no feedback and stay for the next triage pass).

### #808 (`perf(core)`: BHVec10240 xor SIMD)
Two red gates, both self-inflicted: `commitlint` failed on a body whose evidence section lived only in a commit message, and Codacy `⛔ 3 high` flagged `unsafe` in the *newly extracted* test module — those calls were already covered while they lived inside the excluded `hyperdim_simd.rs`. The roast also found 17 rationale/SAFETY comment blocks blanked, an unmeasured `MaybeUninit` + `get_unchecked` fallback (the #783 class), and a cited baseline id (`bhvec_bind`) that does not exist in `canonical.json`.

Fixed on `aee4253`: comments restored verbatim; the `[u64; 160]` dispatch extracted into `hyperdim_simd::xor_u64` beside the identical hamming dispatcher (484/500 LOC — extraction, not deletion); the unchecked fallback replaced by the safe `xor_u64_scalar` oracle; SAFETY comments restored in the test module with four new xor parity tests (dispatched vs oracle on head/tail words, AVX2 kernel vs oracle, `xor` vs `HVec10240::bind` across layouts); `.codacy.yml` covers the extracted module; the body carries measured evidence (287.6 → 141.5 ns per `xor`, ≈ 1.7–2.0×, sign-consistent over alternating rounds). `commitlint`, Codacy and `Test Workspace Crates (csm-core-lib)` are green on the new head; impact is public-API-only, so the merge call stays with the owner.

### #800 (`perf(core)`: permute zero-shift fast path)
The 2026-10-01 evidence request stood: no bench reached the new `word_shift == 0` branch (the only permute bench used shift 321). Added the `hvec_permute` group (shifts 0, 10240, 10240 × 3 — all identity rotations — plus 128 and 321 as controls, `black_box` on vector and result), extended `test_permute` to lock `permute(10240)` and `permute(10240 × 3)`, and measured with an order-alternating harness: 81.7 → 57.3 ns on the direct call (6/6 rounds, control offset reported), 2 684 → 2 734 ns on `text_encoder/encode_short` (3/6 each way — noise: one identity permutation per token stream amortizes away). Gate, Codacy, workspace crates and the arm64 job are green on `79f51f7`; merge/close stays with the owner.

Records: `plans/PR_ROAST_2026_10_02.md` (addendum); lessons in `progress/LEARNINGS.md` (body-capture timing for PR gates; control workloads and alternating order for micro-benchmarks).

## 2026-10-02: WASM freshness gate repaired + root-`src` PR coverage

`main` was red on exactly one job: `wasm`. Runs 36885556956 (`c8f91876`) and 36888300162 (`debd7354`) both failed at `Verify WASM TS Freshness` with a diff made solely of three compiler-generated closure helpers — `wasm_bindgen_847cf8bb5373c819___convert__closures…` — because the checker filtered only the pre-hash naming scheme (`wasm_bindgen__convert__closures_____invoke__…`). Nothing else in those runs failed (the arm64 CLI cancel follows the failed run). The PR-side path filter is why root-source breakage reached the trunk: the `wasm` job set `wasm=true` only for `^(crates/csm-wasm/|src/wasm|wasm/|Cargo\.(toml|lock)$)`, and `src/wasm` is a prefix that never matched root `src/**` — even though `crates/csm-wasm` depends on the root crate with feature `wasm`.

### Change (PR #809, merged `77a47d4`)
- `scripts/check-wasm-freshness.sh`: the filter is anchored at the declaration start and matches generated closure-invocation declarations under both naming schemes — `^[[:space:]]*readonly wasm_bindgen(_[[:xdigit:]]+)?__+convert__+closures__+invoke__`. Public exports, `InitOutput` members and `__wbindgen_*` ABI signatures are still compared; the checked-in snapshot was deliberately not refreshed, because the observed diff is compiler-generated churn rather than API change.
- `.github/workflows/ci.yml`: the `wasm` job's PR path filter now covers root `src/**`, `scripts/{build-wasm,check-wasm-freshness,wasm_size_gate}.sh` and `ci.yml` itself.

### Verification
- Normalization regression executed against the checker's own `filter_dts` (definition read from the script, declaration text on stdin): the job-log fixture (three old helpers replaced by the three hash-prefixed ones) normalizes identically to the checked-in file and is rejected by the previous filter; four API/ABI mutations from that fixture — `encode_text(text: number)`, removed `initialize_wasm`, added `newly_added_api`, `__wbindgen_malloc` arity — all still change the normalized output.
- Path filter, one file per PR diff: `wasm=true` for `src/framework_cleanup.rs`, `src/framework.rs`, `src/wasm.rs`, `crates/csm-wasm/src/lib.rs`, `wasm/test.js`, `Cargo.toml`, `Cargo.lock`, `scripts/check-wasm-freshness.sh`, `.github/workflows/ci.yml`; `wasm=false` for `README.md`, `plans/ACTIONS.md`, `scripts/check-llms-sync.sh`.
- Local end-to-end (wasm-pack 0.15.0, wasm32 target, Node 22.23.2; nix unavailable, canonical commands run directly): freshness `OK`; `build-wasm.sh release-web` 655 497 B; `cargo check --target wasm32-unknown-unknown -p csm-wasm` ok; size gate pass (640.13 KiB, ceiling 800 000 B); `wasm/test.js` → `WASM smoke test passed.`; `shellcheck` clean; `actionlint` findings identical to `main`'s three pre-existing ones.
- PR #809 (head `d0536879`): whole run 36980374291 green, including the previously red `wasm` job — `Verify WASM TS Freshness` → `OK`, `WASM JS Smoke Test` → `WASM smoke test passed.`; merged as `77a47d4`; main push run 36983336327 green. Local wasm-pack emits the old helper names, so CI is the end-to-end proof of the hash-prefixed form.
- Queued action `trigger_wasm_job_on_root_src_changes` completed (queue 9 → 8); `validated` stays false with the remaining wave-32/33 residuals.

## 2026-10-01 (wave-32 residual): Absence record invalidation

Executed `add_absence_invalidation_semantics` (PR #804, merged `c8f91876`) — the audit's F1 remainder: absence records short-circuited queries forever, including after matching content existed.

### Design
- `AbsenceEntry` gains `namespace` + `namespace_revision`; a record short-circuits only when it belongs to this namespace and its revision equals the current one, so every durable mutation (which bumps the revision, ADR-0093) invalidates absence knowledge lazily — no write on the mutation hot path and no list of mutation paths to keep in sync.
- `AbsenceStore::delete_absence` clears the record a probe found once that probe retrieves successfully; the delete only happens when a same-namespace record existed, so successful probes without a record pay nothing extra.
- `merge_absence_with` restarts the counter when namespace/revision changed; records are stamped with the revision observed *before* retrieval; an unreadable revision fails closed; a failed write-path read stamps 0 (inert, matches no live namespace).
- Migration v12 adds both columns behind the guarded versioned `ALTER` pattern (ADR-0021); pre-existing rows default to `('', 0)` and are inert.
- `lookup_absence` replaces `is_known_absent`, returning the decision plus the invalidation target from the single read the check already performs; the mutation exclusion list follows the rename.

### Verification
- `tests/bm25_absence_short_circuit.rs` 3 passed, including the new regression (abstain 3×, short-circuit, inject matching text, probe again → Success); with the revision comparison removed the test fails with `got ["AbsenceShortCircuit"]` — the original bug.
- `cargo test --lib` 165 passed (3 `lookup_absence` boundary/staleness/namespace cases, the `namespace_revision` read test, and the record/clear loop test); `cargo test -p csm-persistence -p csm-traits --all-features` passed.
- v11 → v12 upgrade: a full database parked at v11 (columns dropped, version row removed, legacy row inserted) opens cleanly — `schema_version=12`, the legacy row preserved and inert, a fresh probe records a `_default` attempt instead of short-circuiting, and re-opening is idempotent.
- Four CI iterations, each root-caused: stale `llms*.txt` (regenerated, helpers moved so no phantom `impl` block is emitted); three `ChaoticSemanticFramework::namespace_revision` mutants (killed by a `--lib` test, 62.5% → 80%); two write-path mutants `persist_absence_record`/`clear_absence_record -> ()` (killed by the loop test, both simulated locally first → **100%**); and a `csm-persistence` unit-test initializer missed because `--all-targets` is *package* scope, not workspace scope (fixed; lesson recorded).
- CI on `abff706`: 0 failures — lint, test, mutation-test (100%, 16 mutants: 10 caught / 0 missed / 6 unviable), miri, all nine workspace crates, Cargo Deny, commitlint, benchmark-small.

### State
- `plans/GOAP_STATE.md`: `absence_records_invalidated_on_insert: true`; `queued_actions_count` 10 → 9; `action_last_completed: add_absence_invalidation_semantics`; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed, completion note added.
- `progress/LEARNINGS.md`: the revision-contract lesson for derived negative knowledge, and the `--all-targets`-is-package-scope scope trap.

## 2026-10-01: PR roast triage (#800)

One open PR at triage time: #800 (`perf(core)`: zero-shift fast path for `HVec10240::permute`), a draft Jules PR. Verdict: **keep open, request evidence** — correct and reachable, but unmeasured; record in `plans/PR_ROAST_2026_10_01.md`.

### Findings
- CI truth on `e5333ca5`: every job green except `commitlint`, whose annotation is the perf gate itself — `perf PR evidence gate: missing '## Performance Evidence' section in the PR body`.
- The fast path is byte-identical for `shift ≡ 0 (mod 10240)` and already locked by `crates/csm-core-lib/src/hyperdim_tests.rs:48`; so this is not a correctness objection.
- It is **reachable**, which is why it was not closed as a no-op: `crates/csm-core-lib/src/encoder.rs:226` permutes `pos * position_stride` with `pos` from `.enumerate()`, so position 0 hits the new branch once per `encode`.
- The only `permute` benchmark (`benches/binary_benchmark.rs:30`) uses `permute(321)`; `321 % 128 = 65`, so it never enters the `bit_shift == 0` path — it cannot measure this diff. The body's "~0 ns vs ~144 ns" has no artifact and matches the shape of a harness where LLVM elides the dropped copy.
- Evidence bar posted: `## Performance Evidence` in the body naming `text_encoder/encode_short` (nearest canonical comparator) or the new bench output, an affected-path benchmark with shift `0`/`10240` or the caller `encode`, `black_box` on the returned vector, and the whole-`encode` delta; plus rebase + ready-marking before any merge.
- Lesson distilled into `.agents/skills/pr-roast-triage/SKILL.md`: a bench can be blind because of its arguments, not only because it calls a different function.

### State
- `plans/PR_ROAST_2026_10_01.md`: verdict record. `plans/ACTIONS.md`: triage note. `plans/GOAP_STATE.md`: `action_last_completed` set to the triage action (no queued action was in scope; counters unchanged).

## 2026-10-01 (wave-32 residual): TTL cleanup task ownership

Executed `own_ttl_cleanup_shutdown` (PR #801, merged `96144703`), the audit-F2 remedy: cooperative ownership replaces `abort()`.

### Change
- The task handle is a shared `CleanupTask { cancel: watch::Sender<bool>, handle: Mutex<Option<JoinHandle<()>>> }` in `src/framework_cleanup.rs`; the loop exits on cancel **or** when the last handle drops, because it runs on a framework clone with `cleanup: None` and therefore cannot keep its own sender alive.
- `impl Drop for ChaoticSemanticFramework` is deleted: there is no strong handle in the loop to abort, and dropping one clone no longer touches the shared task.
- `ChaoticSemanticFramework::shutdown()` cancels, awaits under a 5 s bound, is idempotent, works from any clone, and reports a panicked/stuck task as `MemoryError::External`. It is a no-op on `wasm32` (identical public API); the task type and field are `cfg(not(wasm32))`.
- ADR-0099 records the decision; ADR-0024's stale "proposed in ADR-0093" pointer is corrected (ADR-0093 never mentions the lifecycle) and the registry counts move to 95/94. `llms.txt`/`llms-full.txt` regenerated.

### Two CI iterations (both root-caused, not patched)
- `lint` failed the 500-line gate (`src/framework.rs` reached 533) → child-module extraction per AGENTS.md step 9 (now `framework.rs` 476, `framework_builder.rs` 441, `framework_cleanup.rs` 174).
- `mutation-test` failed with 3 missed mutants (whole `spawn_cleanup_task` replaced with `()`, `interval == 0` flipped to `!=`, `shutdown` replaced with `Ok(())`) → killed with two in-crate unit tests asserting that interval 0 spawns nothing, interval > 0 spawns a task, and `shutdown()` awaits the task and takes its handle exactly once (no `--exclude-re` exemption was added). Next CI run: `mutation-test` pass (7m9s).

### Verification
- `tests/test_advanced_ttl.rs` 20 passed, including `background_cleanup_runs_until_shutdown` (purges, then stops after shutdown; idempotent) and `dropping_a_clone_keeps_the_cleanup_task_running` (regression for the clone-drop abort).
- 2 new lib unit tests passed; `tests/arch_fitness.rs` 6 passed (both LOC gates); `cargo check --all-features --all-targets`, `clippy --all-targets --all-features -D warnings`, and `cargo check --target wasm32-unknown-unknown -p csm-wasm` all exit 0; ADR parity ok (registry=95, disk=94).
- CI on `49b6a74`: `lint`, `test`, `mutation-test`, `miri`, the workspace crates, `benchmark-small`, and `commitlint` all pass.
- The host's `target/` directory (83 GiB) was cleaned afterwards at the operator's request; CI is the gate of record from here.

### Findings queued (not fixed in this action)
- `ci.yml`'s `detect-changes` sets `wasm=true` only for `^(crates/csm-wasm/|src/wasm|wasm/|Cargo\.(toml|lock)$)`, so a root-`src` change that alters `cfg(not(wasm32))` gating skips the `wasm` job entirely (observed on PR #801; covered locally instead).
- No caller uses `shutdown()` yet: the MCP server and CLI watch rely on dropping the framework.

### State
- `plans/GOAP_STATE.md`: `ttl_cleanup_has_bounded_shutdown: true`; ADR counts 95/94; `queued_actions_count` 9 → 10 (one completed removed, two findings queued); `action_last_completed: own_ttl_cleanup_shutdown`; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed, two follow-ups queued, completion note added.

## 2026-09-30 (wave-32 residual): crates.io publish pre-check

Executed `fix_crates_publish_precheck_and_add_duckdb` (PR #798, merged `71a48618`): three defects in `publish-crates`, each reproduced against the live registry before the fix.

### Change
- **Ownership pre-check** replaces the version comparison: `/api/v1/crates/<name>/owners` (public; `200` carries `users[].login`, `404` = free) with a descriptive UA. A name passes when it is free or owned by `d-o-hub`; any other HTTP outcome fails closed. `csm-duckdb` joined the checked set (8 names).
- **Version existence** now comes from the sparse index `verify-release` already trusts; the root "already published" check fails closed if the index is unreadable, and the per-crate skip uses the same probe instead of `cargo search`.
- **Companion loop** captures `cargo publish` output, treats "already uploaded" as an idempotent skip, and collects any other failure to exit non-zero — partial publishes can no longer pass silently.
- **`csm-duckdb`** gets a dedicated step after `Publish to crates.io`: skip if published, otherwise wait (bounded 10 min, then fail) for the root version in the index and publish. Its dev-dependency on the root crate is why ordering matters (`crates/csm-duckdb/Cargo.toml:25`).

### Verification (the workflow's shell was extracted and executed, not read)
- Ownership pre-check, real eight-name list: all eight `✅ owned by d-o-hub`, exit 0.
- Negative control: `serde` → `❌ CONFLICT: serde is owned by [dtolnay github:serde-rs:publish]`; unknown name → `✅ name is free`; exit 1.
- `crates-check`: `0.3.8` → `already-published=true`, `0.3.9` → `already-published=false` (both exit 0). The retired probe reads the published 0.3.8 as "not published" (default UA → 403, no match) — the defect.
- `csm-duckdb` step on the published 0.3.8: early exit, no publish attempted.
- `shellcheck -s bash -S info` on the new scripts: 0 findings; `actionlint` 52 findings before → 50 after (the two removed are inside the rewritten step); `cargo metadata` resolves `-p csm-duckdb`.
- Gates: `CARGO_BUILD_JOBS=2 ./scripts/validate.sh` exit 0, `cargo deny check` ok, commitlint 0 problems, CI green on `3d39510`.

### State
- `plans/GOAP_STATE.md`: `crates_publish_precheck_ownership_aware: true`, `csm_duckdb_in_release_publish_order: true`; `action_last_completed: fix_crates_publish_precheck_and_add_duckdb`; `queued_actions_count` 10 → 9; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed; completion note added.
- `progress/LEARNINGS.md`: the three 2026-09-27 supply-chain bullets now carry their resolutions.

## 2026-09-30 (wave-32 residual): Release gate moves to workflow_run

Executed `migrate_release_wait_for_ci_to_workflow_run`. `release.yml` no longer triggers on a main push and polls `gh run list` under a `MAX_WAIT` ceiling (raised 1800 → 2700s, abandoned a third time on run 36031855839 while CI sat `queued` ~40 min on a saturated runner pool); it now triggers on the **completion of the CI workflow** (`workflow_run` on `ci.yml`, `types: [completed]`, `branches: [main]`) and releases exactly `github.event.workflow_run.head_sha`.

### Change
- `wait-for-ci` (83 lines of polling + queue-starvation re-trigger) deleted; `validate` is the entry job, gated on `conclusion == 'success'` **and** `head_repository == github.repository` **and** `event == 'push'` — a fork PR branch named `main` cannot reach a job holding `contents: write`.
- Checkout, the tag created by `validate`, and the GitHub release `target_commitish` all pin `head_sha` (new `release-sha` output) instead of `github.sha`, which under `workflow_run` is the default-branch tip rather than the commit whose CI passed.
- `workflow_dispatch` keeps the fail-closed rule with one query: the new guard refuses to proceed unless CI for HEAD is already `completed`/`success`. `actions: write` is dropped; `validate` adds only `actions: read`.
- Docs trued: `release-management` skill (hard rule 3 + flow) and its `release-workflow.md` reference, `agents-docs/release-safety.md`, `.agents/context/shared-conventions.md`; `progress/LEARNINGS.md` records the durable rule.

### Verification (merged `8c1f864d`, PR #796)
- `actionlint .github/workflows/release.yml` → no expression/context/schema errors (remaining output is the pre-existing `ubuntu-24.04` label-data lag and SC2086 notes in untouched steps); `yaml.safe_load` → `workflow_run` + `workflow_dispatch`, 9 jobs, no dangling `wait-for-ci` reference.
- **Trigger proof:** CI on `8c1f864d` completed `success` (run 36743298010, ~28 min queued) and release run **36746741481** fired with `event: workflow_run` (16:47:13Z); `validate` = success, every publish job `skipped` (tag `v0.3.8` exists → `release-needed=false`), `notify` = success. No `push`-triggered release run exists for that SHA (the old path is gone).
- **Dispatch proof:** `gh workflow run release.yml` → run **36746882951** `validate` = success, log shows `CI on 8c1f864d495c27d321a7339526530baeb853ed8f: status=completed conclusion=success` and `Tag v0.3.8 already exists; skipping release.`
- `CARGO_BUILD_JOBS=2 ./scripts/validate.sh` exit 0 (incl. skill-format validation of the edited SKILL.md), `cargo deny check` ok, commitlint 0 problems, CI on `9bca25e2` all green.

### State
- `plans/GOAP_STATE.md`: `release_wait_event_driven: true`; `action_last_completed: migrate_release_wait_for_ci_to_workflow_run`; `queued_actions_count` 11 → 10; `main_head` refreshed.
- `plans/ACTIONS.md`: action removed; completion note added.

## 2026-09-30 (wave-32 exit pass): Flag truth vs the July audit

Re-verified every exit criterion of `plans/GOAP_AUDIT_2026_07_14.md` against HEAD `7ce6fb73` instead of trusting recorded flags. Three read-only evidence sweeps covered Phases 1-4; every claim below was re-checked by command or by reading the cited source.

### Met (with evidence)
- ANN config fallible: `validate_index_backend` (`crates/csm-memory/src/index/mod.rs:113`) is called from `FrameworkBuilder::build` (`src/framework_builder.rs:354`); invalid HNSW/LSH configs return `InvalidInput` (unit tests in `index/mod.rs`).
- Stale snapshots rejected: `IndexSnapshotEnvelope` carries `namespace_revision` + `backend_fingerprint` and is applied only on exact match (`src/framework_persistence.rs:131-134`); `tests/ann_revision_envelope.rs` (`stale_snapshot_after_inject_is_rejected_on_reload`, `backend_mismatch_rejects_snapshot`).
- Fuzz workspace compiles: `cargo check --manifest-path fuzz/Cargo.toml --all-targets --locked` exit 0 locally; CI job `fuzz-build` (`ci.yml:603-627`) runs the identical command.
- Lean feature matrix: 0 libsql / 0 rayon on normal+build edges (`cargo tree -p chaotic_semantic_memory --no-default-features -e features,normal`); the remaining `rayon` hits are dev-only via `criterion`.
- One WASM artifact: CI (`ci.yml:521`) and release (`release.yml:618`) both call `scripts/build-wasm.sh release-web`; `scripts/wasm_size_gate.sh` measures that artifact (656 657 B threshold, sha256 printed).
- Workspace/supply-chain CI: whole crate matrix (incl. `csm-chaos`), `cargo-deny`, `test-benchmarks`, `mutation-test`, `miri`, `test-duckdb`, arm64, `fuzz-build`.
- Constant-query load and lock discipline: `load_all_associations` (`crates/csm-persistence/src/persistence_index.rs:148`) used by `load`/`load_replace`/`load_merge` (`src/framework_persistence.rs:100,184,304`); durable I/O happens before singularity locks are taken (`:96`, `:177-215`).
- Metric math: true multi-label `recall_at_k`, log2 NDCG, abstention gold from `should_abstain`, hand-calculated tests (`benchmarks/src/scorer.rs`); `cargo test --manifest-path benchmarks/Cargo.toml --locked` 31 passed.
- Measured (not formula) memory claim: `tests/performance_targets.rs:69` measures on-disk bytes/concept (2 875 measured, band 2 500-3 500) plus RSS; `plans/evidence/scale_release_2026_09_21/memory_model.json` records the 12 MB target as not supported.
- Absence short-circuit wired: `src/framework_ttl.rs:170-195,252`, `ABSENCE_MIN_ATTEMPTS = 3`, `tests/bm25_absence_short_circuit.rs`.
- No missing implementations: 0 hits for `TODO|todo!|unimplemented!|FIXME` in `src/` + `crates/`.
- Unique ownership: facades verified (`src/retrieval/bm25.rs` shim, `src/embedding/mod.rs`, `src/persistence_wasm.rs`), canonical test owners per `plans/TEST_SURFACE_AUDIT_2026_09_18.md`; inventory 1037 unique tests.
- Plans compact: `GOAP_STATE.md` 9.5 KB / `ACTIONS.md` 18.1 KB (≈100 KB / ≈180 KB at audit time), with `plans/ARCHIVE_MANIFEST.md` + `plans/README.md` redirects.

### Remainder (queued as nine actions)
- TTL cleanup shutdown is `Drop`-time `abort()` on an `Arc`-shared handle (`src/framework.rs:44-50`): no cancellation token, no await, any clone drop kills the task for all clones, and no test sets a non-zero cleanup interval.
- Absence records are never invalidated on insert or on later success (stale short-circuit).
- No persistence failure-injection test and no query-count test for the bulk association load.
- Gate graph fragmented: orphaned validators (`validate-workflows.sh`, `validate-git-hooks.sh`, `validate-links.sh`), fixture tests (`test-llms-sync.sh`, `test-version-sync.sh`) invoked by nothing, three hook installers with different sets.
- Skill catalog is hand-written and stale (32 vs 33); `scripts/gen-agents-context.sh` and `docs/architecture/context.yaml` carry stale counts (`active_wave: 11`, `total_tests: 134`, `skills: 19`, `adrs: 16`).
- Evidence tiers: no scheduled scale tier, `pre-release-gate.yml` uncalled and bench-free, `benchmark-ci.yml` path filters miss `crates/**`; mutation has no module-level inventory and still excludes changed files.
- `ci.yml` crate matrix is hand-maintained; archive manifest has no validator and omits the 55 top-level archived ADRs.

### State
- `plans/GOAP_STATE.md`: `validated` stays `false` with the evidence-based residual list; `wave_32_status` stays `in_progress`; `main_head` refreshed; +`no_state_lock_across_io_await`, +`benchmark_metrics_mathematically_correct`; annotations on `persistence_failure_leaves_memory_unchanged`, `workspace_ci_matrix_complete`, `plan_archive_manifest_valid`; `integration_test_files` 71 → 72; `queued_actions_count` 3 → 11.
- `plans/ACTIONS.md`: completed action removed, nine residuals queued, completion note added.

## 2026-09-30: Remaining PR impact review

Closed #790 as an incorrect mixed-scope submission and #793 as its incorrect single-list duplicate after posting roast recommendations. Neither supplied the required PR-body performance evidence; both select the wrong candidate for negative public weights. The independent HashMap proposal remains eligible for a measured atomic resubmission, not branded a proven no-op. Keeper review found and corrected #791's missing `clear_associations` invalidation, #789's overbroad codegen claims, and #792's codec/MSRV wording. See `plans/PR_ROAST_2026_09_30.md` for verdicts and sequential merge discipline.

### Merge execution
- #791 (cache invalidation + capacity-safe upsert) squash-merged as `cd70fa5` after 27/27 checks green; `cargo test -p csm-memory --lib` 67 passed. #789 (roast record + skill lesson) rebased at the `PROGRESS.md` top anchor and merged as `977f36f`. #792 (recommendations + `PR_ROAST_2026_09_30.md`) rebased onto that and merged as `195d6e61`. Every keeper was rebased onto the current `main` and re-verified on the exact new head before an explicit squash merge; no `gh pr merge --auto`.
- Queue after the wave: 0 open PRs, 0 open issues. Inventory 1037 unique compiled tests; LOC max 500; ADR parity and llms-sync gates green.

## 2026-09-29: Retrieval cache and capacity invariants

### Summary
Implemented the first atomic recommendation from the 2026-09-29 audit in the owning `csm-memory` crate. New concept inserts now invalidate cached similarity results; association, disassociation, and clear-associations mutations invalidate graph-derived retrieval caches; and upserting an existing ID no longer evicts an unrelated concept when the store is at capacity.

### Verification
- Added regression tests for insert-after-cache, capacity-safe upsert, and graph candidate cache invalidation after association, disassociation, and reconnect/cache/clear/query.
- `cargo test -p csm-memory --lib`: 67 passed.
- `cargo clippy -p csm-memory --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.

### State
- This is intentionally a small correctness slice; atomic imports, scoped absence records, TTL shutdown, snapshot codec migration, and backup/restore completeness remain separate work.

## 2026-09-28: PR Roast Triage (#783–#786)

### Summary
Session-start roast of all four open PRs. One PR closed with its roast comment
for no demonstrated zero-fill-elimination benefit; three dependabot keepers
verified and a manual merge order emitted (no
merges executed by the triage). No queued GOAP action was in scope;
`action_last_completed` unchanged.

### Actions
- #783 (`perf(core-lib)`: BHVec `MaybeUninit` zero-fill elimination, Jules
  draft): **closed as no-op**. Old-vs-new codegen probe (x86_64, rustc 1.98.1,
  `-O -C codegen-units=1`, `#[no_mangle] #[inline(never)]` linked binary):
  `from_hvec`/`from_bytes` variants fold to the same address (byte-identical
  code); an old-only build disassembles to a bare `memcpy` — LLVM already
  dead-store-eliminates the zero-init. `to_bytes` old/new share the same
  alloc+memcpy sequence but differ in instruction scheduling. No demonstrated
  zero-fill-elimination benefit; the scheduling change means disassembly alone
  cannot dismiss all claimed timings as noise. A scheduling-speedup claim needs
  reproducible affected-path benchmarks. The perf-evidence gate was red
  (no `## Performance Evidence`), commit 1 scope
  `core-lib` invalid (not in `commitlint.config.cjs`), draft behind `main`.
- #784 (install-action 2.87.15 → 2.87.20): keeper. SHA
  `9983c65e42da123ff25d1f78505eb6de315aa172` equals tag `v2.87.20`'s commit;
  the only install-action pin in `.github/workflows/`; 18/18 checks green.
- #785 (patch-updates: rmcp 3.4.1, hyper-util 0.1.21, thiserror 2.0.21,
  rand 0.10.3): keeper. Lock-only diff; 32/32 checks green; `tempfile`'s
  `getrandom` edge re-resolves 0.4.3 → 0.3.4 within its `>=0.3.0, <0.5`
  requirement (both majors remain in the graph); no deny.toml/export.json noise.
- #786 (rand 0.10.3 in `/fuzz`): keeper. Fuzz lock only; head current
  (`MERGEABLE`/`CLEAN`); all checks green.
- Record: `plans/PR_ROAST_2026_09_28.md`; merge order #786 → #784 → #785, each
  rebased onto current `main` and CI re-verified on the new head before an
  explicit squash merge.

### State
- `plans/ACTIONS.md` / `plans/GOAP_STATE.md`: unchanged (triage is not a queued
  action).
- Lesson distilled into `.agents/skills/pr-roast-triage/SKILL.md`;
  `progress/LEARNINGS.md` updated.

## 2026-09-27 (wave 3): Borrowed-Id Retrieval Expansion (#781)

### Summary
Executed `eliminate_retrieval_string_clones`. Measurement first: the queued hypothesis (#754 roast) said `score_specific_candidates` clone-per-candidate dominated the retrieval path — a counting allocator showed the function at n=256 costs 263 allocations, while a default bridge query allocated 2 209, dominated by `ConceptGraph::match_tokens`/`expand` materialising every matched id and expanded label before the caller truncates to top_k. The fix therefore borrowed the whole path: `match_tokens_ref`/`expand_ref` (csm-traits), `score_candidate_positions` (csm-memory), generic `normalize_scores_in_place` (csm-retrieval), `&str`-keyed merge (root bridge). All owned public signatures kept as delegating wrappers — no breaking change.

### Measured
- New `bridge_retrieval/pipeline_1k_expansion` bench (one query token matches all 1000 concepts): criterion medians 881.26 µs → 577.07 µs, change −41.6% [−47.9, −34.4], p < 0.05; reversed A/B/A leg old-vs-new +28.8% [+19.7, +38.1].
- Allocations per query (counting allocator, deterministic): 2 209 → 151 at top_k=10 (−93%), 3 820 → 1 228 at top_k=101, 3 115 → 724 at top_k=64. Public `score_specific_candidates` wrapper unchanged (263 → 265, control).
- Comparator caveat: the cached `pipeline_1k_concepts` bench drifted 9.1–13.4 µs across windows (±30%) — treated as noise; one large-sample A/B/A leg discarded (first leg 1.30 ms, outside every other window).

### Actions
- PR #781 merged (`6194fca`) after CI green (lint, test, mutation-test, miri, benchmark jobs) and the roast verdict comment; record appended in `plans/PR_ROAST_2026_09_27.md`.
- Tests: `test_concept_graph_ref_variants_match_owned`, `score_candidate_positions_matches_owned_wrapper`; `validate.sh` and `harness-check.sh all` green (documented link-OOM mitigation); LOC max 500.

### State
- `plans/ACTIONS.md`: action removed; `plans/GOAP_STATE.md`: `retrieval_string_clones_removed: true`, `action_last_completed: eliminate_retrieval_string_clones`, `queued_actions_count` 4 → 3, `main_head` refreshed.

## 2026-09-27 (wave 2): csm-duckdb First Publish

### Summary
Executed `publish_csm_duckdb_companion`. `csm-duckdb` was the last unpublished companion crate (name free; `chaotic_semantic_memory` and `csm-chaos` owned by `d-o-hub`). Readiness before the irreversible step: `cargo metadata --locked` clean, `cargo package --list` reviewed (tracked test fixtures only), and a full `cargo publish --dry-run -p csm-duckdb` (package extracted and compiled from the tarball, 13m25s). Published `csm-duckdb 0.3.8` at 2026-09-27T14:07:08Z (38 356 B); verified via `cargo search` (0.3.8), `cargo owner --list` (`d-o-hub`) and the crates.io API.

### Findings (queued as `fix_crates_publish_precheck_and_add_duckdb`)
- `cargo publish` verification resolves dev-dependencies against the registry (probe crate: dev-dep `serde = "99"` fails package prep). `csm-duckdb` dev-depends on `chaotic_semantic_memory`, so it must publish *after* the root crate; the release loop publishes companions first and swallows failures — appending it to that loop would silently skip it every release.
- `release.yml`'s name-availability pre-check reads our own older version as an unrelated-project conflict: simulated against the live registry at 0.3.9 it sets `NAME_CONFLICT=true` and exits 1 for all seven published companions — the next version release fails before publishing anything.
- The `crates-check` `curl` probe receives a 403 (Fastly) with curl's default UA — no `"version"` key in the body — so the "already published → skip" short-circuit cannot be trusted as written (a descriptive UA returns JSON).

### State
- `plans/ACTIONS.md`: `publish_csm_duckdb_companion` removed; `fix_crates_publish_precheck_and_add_duckdb` queued; `queued_actions_count` stays 4.
- `plans/GOAP_STATE.md`: `duckdb_companion_published: true`; `action_last_completed: publish_csm_duckdb_companion`; `main_head` refreshed.

## 2026-09-27: llms Dependency Versions Synced + Drift Gate

### Summary
Executed `regenerate_stale_llms_dependency_versions`. The committed `llms.txt`/`llms-full.txt` were last regenerated at #714 and still reported `opentelemetry 0.27`, `tracing-opentelemetry 0.28` and `rmcp 1.7` while the manifests carry `0.32`/`0.33`/`3.4` (bumps #728/#729), plus the `scale_evidence` example added in #733. `scripts/validate.sh` regenerated the files but never committed or compared them, so CI could not see the drift. Both files were regenerated through the pinned v0.1.1 generator (no hand-edits) and the validate step is now a failing drift gate.

### Actions
- `scripts/check-llms-sync.sh`: snapshots both files into a `mktemp -d` dir (EXIT trap), runs `scripts/gen-llms-txt.sh`, `cmp -s` each and prints `stale generated file: <name>` per drifted file (exit 1); missing inputs fail with `missing generated file: <name>` instead of an empty baseline; generator failure propagates its exit code. Regenerated files stay on disk for review.
- `scripts/validate.sh` calls the checker in place of the bare generator (public-API LOC check unchanged); pre-commit/release/`sync-docs` regeneration paths intentionally untouched.
- `scripts/test-llms-sync.sh`: five fixture cases in a mock repo with a stub generator — synced passes; stale `llms.txt` and stale `llms-full.txt` each fail and are named; missing input fails; generator failure exits 3. No Cargo/network; ShellCheck clean.
- Verification: `scripts/test-llms-sync.sh` 5/5; checker run twice on synchronized files exits 0 both times and leaves both files byte-identical (`sha256sum`); induced drift exits 1 naming `llms.txt` and aborts a `set -euo pipefail` caller; ShellCheck clean. `./scripts/validate.sh` stopped at the known local link-OOM (`cargo test --no-run --all-features`, `ld` SIGKILL — environment, LEARNINGS 2026-09-23); the same sensors rerun with `CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4`: `cargo test --all-targets --all-features` exit 0, `cargo deny check` ok, `./scripts/harness-check.sh all` green (fmt, workspace clippy, deny, test, arch). ADR parity ok; LOC gate max 500. No Rust sources changed (`./target/debug/csm` behavior untouched).

### State
- `plans/ACTIONS.md`: `regenerate_stale_llms_dependency_versions` removed; `queued_actions_count` 5 → 4; `llms_dependency_versions_current: true`; `action_last_completed: regenerate_stale_llms_dependency_versions`.
- Rule encoded in `agents-docs/hard-constraints.md` (generated listings must not drift; checker + fixture tests named).

### Merge
- Squash-merged as #778 (`4cf4f2c`) from head `b38b87a`, up to date with `main` and CI green (the `lint` job ran the new checker through `validate.sh`); verdict in `plans/PR_ROAST_2026_09_27.md`.

## 2026-09-25 (wave 2): Merge Discipline Bound + Queue Cleared

### Summary
Bound two merge rules that were only implicit, then executed the queue under them. `AGENTS.md` step 18 and Core Rule 8 now require: **never `gh pr merge --auto`**, and **every PR merged from a head that is up to date with the latest `main`** — rebase, push, wait for CI green on *that* head, re-check `mergeable`/`mergeStateStatus`, then merge; repeat per PR because each merge moves `main`. The roast skill's Step 5 became "emit, do not execute unless instructed" with the mandatory per-PR command sequence. The rule is not theoretical: a bot branch's rebase resolved `progress/PROGRESS.md` toward its pre-fork side and would have silently reverted **787 lines** of merged work (the gate script, its `ci.yml` step, the PR template section, the `AGENTS.md` roast rule, both `PR_ROAST` records) had it been merged on the green tick.

### Actions
- Merged in order, each one rebased onto the live `main` tip and CI-verified on its post-rebase head immediately before landing: **#773** merge discipline (`f2eee21`), **#771** evidence-gate scope trace (`835b28e`), **#772** doc-contract rule (`b4ee04e`), **#767** `chaos_strength` clamp (`25b9092`), **#774** closure records (`554faeb`). A sequencer enforced the discipline; it stopped once on a head-moved guard rather than merging on an uncertain head.
- Closed four perf PRs as **no demonstrated impact**: **#763** and **#769** (owner call), then resubmissions **#775** and **#776**. `#775` re-proposed the closed #763 change and quoted "up to ~13.5 %" — the **best row** of the sign-flipping table from my own #763 measurement (three losses, including +8.9 % at K=100). `#776`'s single-token fast path is behaviour-preserving (verified by digest equality on both revisions) but its `Vec::with_capacity` hunk is **byte-identical** in allocations (1 alloc, n × 1 280 B, both variants — the #754 pattern recurring), and its long-text claim has no mechanism in the diff.
- #767 landed clean: clamp, flipped integration test asserting the clamp, unit test, and the two restored behaviour contracts.

## 2026-09-25: Three-PR Roast (#767, #768, #769)

### Summary
Second roast pass under the gate bound in `AGENTS.md` (merged `31f793f`). Three drafts, three different verdicts — and the first time the rubric's own rules were tested against myself: I initially read #767's 21-line doc removal as comment-stripping, then measured it (`grep -c "ADR-0094"` → 3 → 3) and found the rationale reworded, not lost. The roast says what was actually lost (the `#[cfg(feature = "persistence")]` contract prose on ~7 methods) and not what wasn't. The skill now carries that "verify before accusing" step so the next reviewer doesn't repeat my near-miss.

### Actions
- **#767** `fix(framework): clamp chaos_strength` — **merge after doc restoration.** The fix is real: `with_chaos_strength` passed `NaN`/`±∞`/out-of-range straight to the reservoir; the clamp plus boundary tests is right, and the 20-test builder baseline is green on main. Blocked only on restoring the persistence-feature contract prose, plus a body note for the `const fn` → `fn` API change and the flipped negative-value test expectation.
- **#768** `feat: chaotic complex map` — **closed as submitted.** 10 research scratch files committed to the repo root, a self-marked `[FALLBACK — outside this run window]` citation behind a new public API, and a "~9x faster" headline beside "Not measured" rows for entropy/distribution/recall. The idea is not rejected; the packaging is. Remote branch deleted.
- **#769** `perf(memory): get_unchecked in LSH projection` — **request evidence.** The SAFETY argument is correct (verified: `bit_pos < 10240` ⇒ `byte_idx >> 3 < 1280`, gated on `bytes.len() >= 1280`) and the safe path is retained — materially better than the #737/#739/#740/#754 class. Blocked on the evidence gate (live body → `exit 1`, missing `## Performance Evidence`) and on a real safety point: the bound is a hardcoded literal in a fn generic over `Hypervector`, so a future non-10240 impl becomes an out-of-bounds read, not a perf regression.
- Records: `plans/PR_ROAST_2026_09_25.md` (153 LOC, per-PR verdicts + recommendations); three roast comments posted, with the closing comment on #768; three `progress/LEARNINGS.md` entries; rubric additions to `pr-roast-triage` (verify-before-accusing, unsafe-bounds-in-generic-code, self-contradicting-evidence-tables). `validate-skill-format.sh` 33/33.

## 2026-09-24: Roast Before Implement or Merge (AGENTS.md Rule)

### Summary
Made the PR/issue roast a standing gate rather than a thing we remembered to do. `AGENTS.md` now binds it in three places: Phase 1 step 5 (review and roast before **implementing** an issue/PR — premise, affected code path, evidence bar), Phase 5 step 17 (**no verdict, no merge** — CI green is necessary, not sufficient), and Core Rule 9. A no-impact PR is a terminal outcome: roast comment → close → record `plans/PR_ROAST_<date>.md` → update `progress/` → distill the lesson into `.agents/skills/`; quietly re-implementing a closed PR's idea without the evidence it lacked is the failure mode the rule prevents.

### Actions
- `.agents/skills/pr-roast-triage/SKILL.md` gained a "When this gate applies (MANDATORY)" section (when the roast runs, the terminal no-impact flow, distill-don't-duplicate) — compacted into the existing skill rather than adding a near-duplicate; `scripts/validate-skill-format.sh` passes (33/33).
- `progress/LEARNINGS.md` PR Triage section: one entry recording the #737/#739/#740/#754 + #520 pattern (5 rounds of close-then-guardrail before the rule existed).
- AGENTS.md workflow steps renumbered continuously 1–18 across five phases; Core Rules 1–9; 90 LOC (≤ 200 cap).
- Applied to the live queue: #763 (`perf(retrieval): defer score scaling in single list hybrid merge`, +66/−70, one file, no Criterion/flamegraph, no `## Performance Evidence`) **fails the new CI gate as-is** — it is the exact no-impact class this rule governs.

## 2026-09-23 (perf gate): Perf-PR Evidence Gate Enforced

### Summary
`enforce_perf_claim_evidence_gate` landed: `scripts/check-perf-pr-evidence.py` runs in the always-on `commitlint` job of `.github/workflows/ci.yml` (no `detect-changes` dependency, so every PR is covered regardless of path filters) and, for titles beginning `perf(`, requires a `## Performance Evidence` section documenting a baseline (`plans/evidence/bench/canonical.json` or a benchmark id from it; nearest canonical comparator plus new output when the file has no entry), a path/URL-shaped Criterion output or flamegraph reference (a bare `flamegraph` word does not count), and a numeric before/after pair for the affected benchmark. Title and body reach the checker as positional argv with `PR_TITLE`/`PR_BODY` environment fallback (never shell interpolation of `${{ }}` into `run:`); scope validity stays with commitlint's `scope-enum`, non-`perf(...)` and bot titles exit zero, and the check is presence/shape only — it does not run benchmarks or assert improvement. `.github/PULL_REQUEST_TEMPLATE.md` gained the matching section.

### Actions
- Prerequisite merged first: `fix(core)` #759 (`e5d039e`) converts two `manual_range_contains` asserts in `csm-core-lib` re-export tests to `(0.0..1.0).contains(&v)`. Reason: CI's `lint` job and `validate.sh` both run clippy **root-scope** (no `--workspace`), so those two errors sat invisible to CI while the workspace-wide sensor in `scripts/harness-check.sh all` stayed red on main; the fix unblocks the local harness gate, not CI.
- Nineteen-case local matrix exercised (adds adversarial template cases and the argv interface): non-perf/docs/bot/empty titles pass; missing section, empty body, missing baseline, missing artifact, missing numeric pair, placeholder-only, untouched-template, and "template + numbers only with the artifact prompt unedited" bodies fail with named `::error::` lines; complete Criterion/flamegraph bodies and argv invocations pass.
- `plans/ACTIONS.md` queue drops to three actions (`enforce_perf_claim_evidence_gate` removed), `plans/GOAP_STATE.md` sets `perf_pr_evidence_gate_enforced: true` and `action_last_completed: enforce_perf_claim_evidence_gate`; `validated: false` and the wave-32 reconciliation action stand (the July audit has other exit criteria).
- No benchmark code, threshold or evidence artifacts touched; `benchmark-ci.yml` and `scripts/bench-baseline.sh compare` remain the measurement gates.

## 2026-09-23: Harness-Trial Queue Reconciled (#748-#752)

### Summary
Reconciled `triage_harness_trial_issues_748_752` against landed changes: #752 (libsql vs markdown spike) and #748 (do-harness P0 wrapper, drop-for-P1 verdict parity failure) and #751 (goap-plus-adrs guardrail, hard-constraints.md:44-47) all closed; #749 ranker (`scripts/skills-suggest.sh`, frontmatter-only, no do-harness dependency) plus the four-step loop (`agents-docs/quick-reference.md:6-13`) merged in PR #757 (squash `85e9e89`, all 23 checks green, Codacy included), and all three acceptance queries return the expected skill in top five; #750 commented with the non-adoption decision (keep current `install-hooks.sh` path) and closed, #749 closed (auto-closed on merge). Queue now holds four actions; `queued_actions_count: 4`; `action_last_completed: triage_harness_trial_issues_748_752`.

## 2026-09-22: Perf-PR Roast (#754) + Followup Queue

### Summary
Single open PR triaged: #754 (`perf(memory)`: pre-allocate capacity / drop iterator closures in `singularity_retrieval`) closed as **no demonstrated impact**. Counting-allocator adjudication of every changed collection shape: the two scan-tail hunks are byte-identical before/after (257 allocs / 11 264 B — `TrustedLen` `collect()` is already single-exact-alloc), the `graph_candidates` tail hunk is self-contradicting (`with_capacity(len.min(max))` then pushing `len` before `truncate` re-allocates in the case it claims to fix), and the residual ~13-alloc / ~12 KB growth-chain delta is dwarfed by the 257 `String` clones and 256 × 1 280-byte Hamming scans the rewrite keeps. No criterion evidence attached (4th entry in the #737/#739/#740 class).

### Actions
- Roast comment posted on #754 with the evidence table + resubmission target (`plans/evidence/scale_release_2026_09_21` baselines); PR closed, remote branch deleted.
- Record: `plans/PR_ROAST_2026_09_22.md`; guardrail learning in `progress/LEARNINGS.md`.
- Queued 5 followups in `plans/ACTIONS.md`: harness-trial issues #748–#752 (start: #752 spike, gates #748), `csm-duckdb` publish, perf-claim evidence gate, retrieval `String`-clone elimination, wave-32 flag-truth reconciliation.

## 2026-09-21 (wave 2): Bucketed-Candidate Recall Fixed at Scale

### Summary
Executed the action the Tier-3 evidence queued an hour earlier: the bucketed candidate generator lost recall as the corpus grew. Two distinct defects were separated by measurement — a biased truncation and a fixed probe width — and both are fixed with a policy that declines rather than returning an unrepresentative slice.

### Measured, before the fix
| configuration | 10 k | 50 k | 200 k |
|---|---|---|---|
| `floor 2` (library default) | recall 0.016 | 0.068 | **0.016** |
| `floor 8` (evidence setting) | 0.604 | 0.576 | **0.208** |

All widths from 2 to 6 returned exactly `max_candidates` candidates: the bucket was **truncated in index order**, so the sample was biased (the first clusters only) — that is the dominant loss, not the width itself. `fell_back_to_exact_scan` never fired.

### Fix (`crates/csm-memory/src/singularity_retrieval.rs`)
- `effective_bucket_probe_width(configured, corpus_size, budget)`: the configured width is a **floor**, raised so one bucket fits `max_candidates`, capped at `MAX_BUCKET_PROBE_WIDTH` (16).
- **Multi-probe**: candidates within one masked bit of the query bucket are accepted.
- **Budget guard**: a probe larger than `max_candidates × BUCKET_BUDGET_SLACK` (2) returns nothing, so the caller's exact-scan fallback answers instead of a sliced bucket.

### Measured, after the fix
| configuration | 10 k | 50 k | 200 k |
|---|---|---|---|
| `floor 2` | probe declines (25/25) → recall 1.000 at exact-scan latency | same | same |
| `floor 8` | 570 candidates, **0.832** recall, 249 µs (exact 558 µs) | probe declines (23/25) → 0.976 | declines (25/25) → 1.000 |

The bucketed path can no longer silently return a low-recall candidate set: it either produces a bounded, higher-recall probe or defers to the exact scan. Unit tests cover the width function and the decline/multi-probe behaviour; `bucket_sweep/` in the evidence directory holds every run (pre-fix widths 2–16 at 200 k, post-fix floors 2 and 8 at 10 k/50 k/200 k).

## 2026-09-21: ADR-0095 Tier-3 Evidence — Reference Runner, Canonical Baseline, Release-Scale Artifacts

### Summary
Completed the Tier-3 release-claim requirements and flipped `benchmarks_prove_performance` to true. The gap was concrete: `pre-release-validate.sh` compared Criterion against `--baseline main`, a baseline that lives in git-ignored `target/` and is wiped by `cargo clean`, so no release was ever gated on a measurement.

### Actions
- **Reference runner** (`plans/REFERENCE_RUNNER.md`): names `csm-ref-01` (i5-8350U, 16 GB, Linux, rustc 1.88.0) and the rules — every published number names its runner, cross-runner comparisons use ratios, release claims need a committed baseline or a CI ceiling.
- **Canonical baseline** (`scripts/bench-baseline.sh`): `save` records `plans/evidence/bench/canonical.json` — 88 benchmarks with median and confidence interval plus commit/toolchain/CPU; `compare` re-measures and reports deltas with CI overlap, failing only when delta > tolerance *and* the intervals do not overlap, `--advisory` for laptop runs. Observed noise on `csm-ref-01` justifies that rule: `delete_concept*` moved +27 %/+43 % between two runs minutes apart (non-overlapping CIs), and a later run flagged `crud_roundtrip_percentiles` +22.8 % instead — hence advisory + re-run-to-confirm rather than a hard wall on a laptop. `pre-release-validate.sh` section 8 now calls the comparison.
- **npm artifact evidence** (`scripts/wasm-evidence.sh`): builds `release-web` through `scripts/build-wasm.sh` (which now resolves its out-dir against the repository, since wasm-pack resolves relative paths from the crate), runs the Node smoke test, and records bytes, SHA-256s, toolchain and the transcript in `plans/evidence/wasm_2026_09_21/`. The `.wasm` is byte-identical across three independent builds (`6804814a…`).
- **Release-scale evidence** (`plans/evidence/scale_release_2026_09_21/`): ANN at 10k/50k/200k and memory/storage at 50k/100k/200k fit with a 500k hold-out. Two findings, both new: bucketed candidate recall@10 collapses 0.604 → 0.576 → **0.208** because `bucket_probe_width` is fixed while the corpus grows (queued as `fix_bucketed_candidate_recall_at_scale`), and HNSW recall at fixed `ef_search` falls 0.908 → 0.896 → 0.800. The memory model stays linear to 100k (0.29 % hold-out error) but under-predicts at 500k by 15.6 %, so the per-point measurements — not the extrapolation — are the claim.
- **Rendering**: `scripts/render-scale-evidence.py` derives a scaling-trend table and the two findings above from the artifacts, so the prose cannot drift from the JSON.

## 2026-09-20: WASM Artifact Parity — CI Now Validates What Ships

### Summary
Closed GOAP audit row A5 / the `wasm_ci_release_artifact_identical` flag. CI, the release workflow, the size gate and the TS-freshness check each built a *different* wasm artifact; the shipped one was never built or smoke-tested before publication.

### Measured divergence (before)
| Path | Command | Artifact |
|---|---|---|
| CI smoke | `wasm-pack --dev --target nodejs` | 4 077 828 B `.wasm` |
| Release (npm) | `wasm-pack --release --target web` (+ wasm-opt) | **656 657 B** |
| Size gate | raw `cargo build --release -p csm-wasm` | 1 133 971 B (threshold 1 150 000 B — 1.4 % headroom) |

The gate guarded an artifact nobody ships, CI validated a 6.2× larger dev build, and no job ever exercised the release profile.

### Actions
- **`scripts/build-wasm.sh`** — single source of truth for the package build (`dev-nodejs`, `dev-web`, `release-web`); prints package dir, byte size and SHA-256.
- **CI wasm job** now builds `release-web` through that script, smoke-tests it, and uploads `wasm/pkg` as an artifact; **`release.yml`** calls the same script, so CI and release cannot drift again.
- **`wasm/test.js`** loads either target: for `--target web` it hands the wasm bytes to `init` (Node's `fetch` rejects `file://`) and merges the ESM named exports with the CJS default view. Verified against both packages — the release/web run also exercises the `importFromBytes` round-trip added in #725.
- **`scripts/wasm_size_gate.sh`** measures the shipped artifact (656 657 B, sha256 `6804814a…`, threshold re-based to 800 000 B with ~22 % headroom) instead of the raw cargo output; **`check-wasm-freshness.sh`** builds through the script too.
- **Reproducibility data point**: two independent `release-web` builds produced byte-identical `.wasm` (`6804814a4aea2885a10dd3ab11255a592d73aff3185cf4eab0df6e74443f5d27`), so the npm artifact is reproducible on a fixed toolchain.
- **Docs**: `book/src/wasm.md`, `book/src/release.md` and the `dist-channel-selection` skill now point at the script. Flag `wasm_ci_release_artifact_identical: true`.

## 2026-09-18: Test-Surface Dedup — Queue Emptied, Coverage Methodology Replaced

### Summary
Executed the last queued GOAP action `deduplicate_test_and_source_surfaces`. Three read-only scouts mapped the surfaces, then a mechanical body comparator and hand review sorted real duplicates from look-alikes. 24 facade test bodies went away; six files that earlier analysis called "verbatim duplicates" were *rejected* with evidence and stay. The GOAP queue is now empty (`queued_actions_count: 0`).

### Actions
- **Duplicates removed (24 bodies, workspace attributes 1053 → 1029)**: `src/embedding/mod.rs` 12 (owner: `csm-embedding`), `src/persistence_wasm.rs` 6 (wasm32-only module; its test module referenced an undefined helper and could not compile), `crates/csm-core-lib/src/maps/neural_circuit.rs` 3, plus one each in `src/lib.rs`, `src/export_payload/export_payload_tests.rs`, `src/wasm_ext_tests.rs`, and an empty module in `src/wasm_graph_rag.rs`.
- **Source dedup**: `csm-core-lib::maps::neural_circuit` was a second, unreachable copy of the CDNCM map (`f64::tanh`, unconditional serde) while `csm-chaos` owns it and the root feature forwarded only to `csm-chaos`. It is now a re-export with the feature forwarding to the owner (serde preserved). `csm-core-lib::hashing::chaotic_lsh` was checked and kept — it is a documented wrapper, not a body copy.
- **Fix ported, not deleted**: the root embedding copies carried the `ENV_TEST_LOCK` race fix from PR #700; `csm-embedding` still had the racy pattern with a false "single-threaded test" SAFETY comment. The guard was ported into the owner (4 tests, 8 env mutations) before the facade copies were removed.
- **False positives rejected (6 files)**: `cache_lru_coverage`, `ttl_lifecycle`, `builder_advanced`, `path_validation_errors`, `critical_error_paths`, `batch_ops_coverage` were proposed for deletion but assert distinct behavior (metrics accounting via `probe_batch_cached`, error variants, retrieval after TTL, invalid-config failures). Evidence table in the audit.
- **Coverage methodology (ADR-0095)**: `scripts/update-coverage.sh` (test-LOC/source-LOC ratio, `crates/` blind, rewrote README rows that no longer exist) deleted; `scripts/coverage-report.sh` added — `inventory` (unique compiled behavior per layer, 1029 attributes; 8 same-name leads, all reviewed) and `llvm-cov`/`llvm-cov-all` (line + branch coverage; 74.33 % lines / 64.08 % branches over lib + all integration targets vs 68.39 %/53.17 % for unit-only). Pre-commit hook now runs the fast inventory.
- **Verification**: baseline `cargo test --all-features` exit 0 → after the deletions exit 0; the instrumented full-coverage run executed every target green; `csm-embedding` 15 passed, `csm-core-lib --features experimental-neural-circuit` 75 passed, `csm-chaos --all-features` 20 passed; clippy/fmt clean.
- **Record**: `plans/TEST_SURFACE_AUDIT_2026_09_18.md`.

## 2026-09-17 (wave 2): ADR-0095 Scale Evidence — ANN/Persistence/Memory Measured, Two Actions Closed

### Summary
Executed the two queued ADR-0095 evidence actions. A new release-only harness (`examples/scale_evidence`, driven by `scripts/scale-evidence.sh`) produces machine-readable artifacts with the full ADR manifest (commit, dirty state, corpus version/seed/checksum, command, features, toolchain, hardware, samples, variance). The measurements contradicted two recorded claims, both corrected here: concurrent local writes failed 90 % of the time (now 0 % after bounded retries in `csm-persistence`), and the `< 12 MB for 10M concepts` target is off by ~3 700× because it described an unimplemented design.

### Actions
- **#733 Merged** (`2fc421b`): runner + first artifacts. ANN at 50 k: exact 6.35 ms p50 (recall 1.000), HNSW 896 µs / recall 0.892 / 80 s build / 93 MB serialized, LSH 813 µs / recall 0.836 / 282 ms build, bucketed candidates (probe width 8) 1.83 ms / recall 0.590 with no index. Memory model: RSS `2 880 665 B + 4 691 B × concepts` (held-out error 0.29 % at 100 k), storage `2 850 B/concept` (0.06 %). Claim correction: the formula-only test became a measured footprint test, and the book/context.yaml target is marked not supported. Also excluded the harness from two opengrep rules in `.codacy.yml` (args/current_exe) after Codacy flagged them as high-severity false positives.
- **#734 Merged**: `csm-persistence` sets `PRAGMA busy_timeout = 5000` on local connections and retries transient lock errors (5 attempts, 2–32 ms) on the idempotent write paths. `tests/persistence_concurrency.rs` (8 writers × 25 saves; 8 writers × batch+associations) is the regression guard; the re-measured artifact shows 0.905 → 0.000 raw and 0.470 → 0.000 with caller-side retries, at the cost of higher tail latency while writers wait.
- **LOC gate**: the retry loop wrappers pushed `crates/csm-persistence/src/persistence.rs` to 518 lines, so the gate failed in both the lint and test jobs; fixed by sharing a `with_retry` helper and extracting `persistence_absence.rs` (the AGENTS.md rule: child-module extraction over comment stripping).
- **SonarCloud**: the new renderer took a filesystem path from argv (two path-traversal findings) and had cognitive complexity 36; it now resolves a validated directory *name* under `plans/evidence/` and is split into per-section functions.
- **Learning recorded**: a perf gate that asserts arithmetic is worse than no gate — the 10M/12 MB test passed for months while the shipped representation needed ~1 000× more memory.

### Next
`deduplicate_test_and_source_surfaces` (P3) is the only queued action; `persistence_contention_evidence_current`, `ann_scale_evidence_current`, `measured_memory_model_exists` and `ten_million_memory_claim_evaluated` are now true in `GOAP_STATE.md`, and `performance_claims_have_current_artifacts` flipped to true.

## 2026-09-17: Dependabot Queue Cleared — WASM bincode Fix, rmcp 3.4, opentelemetry 0.32

### Summary
Follow-on to the 2026-09-15 queue landing. Merged the WASM bincode fix (#725), consolidated the entire Dependabot backlog into two PRs (#726, #727), then took the two remaining major bumps (#724 rmcp 2→3, opentelemetry medium advisory) as real migrations (#728, #729) rather than suppression ignores. Zero open PRs at the end; `progress/LEARNINGS.md` carries the new CI/supply-chain lessons.

### Actions
- **#725 Merged** (`0bcbd5c`): `WasmFramework::exportToBytes` wrote with bare `bincode::serialize` while `importFromBytes` read with `DefaultOptions` — browser export→import round trip failed with `string is not valid utf8`. Aligned the writer and added the round-trip assertion to `wasm/test.js`.
- **#726 Merged** (`ebe5190`): consolidated Dependabot updates. `fuzz/Cargo.lock` batch (thiserror, async-trait, wasm-bindgen-futures, clap, uuid + transitives), `taiki-e/install-action` 2.87.11 → 2.87.12 (supersedes #722), and the double-scope fix. #717–#722 closed as superseded.
- **Dependabot double-scope root cause**: `commit-message.prefix: "chore(deps)"` (and `ci(deps)`) combined with `include: "scope"` produced `chore(deps)(deps): …` titles, which fail `scope-enum`. Prefixes reduced to `chore`/`ci`; an ignore rule for existing double-scoped messages added to `commitlint.config.cjs`; the PR-title check now keys on `github.event.pull_request.user.login` instead of `github.actor` (a maintainer's `update-branch` made the actor a human, so Dependabot PRs started failing a check meant to skip them).
- **#727 Merged**: `clap_complete` 4.6.7 → 4.6.9 in `fuzz/Cargo.lock` (Dependabot-native, CI green).
- **#728 Merged**: rmcp 2.2.0 → 3.4.0. rmcp 3 routes results through MRTR enums — `call_tool` returns `CallToolResponse`, `read_resource` returns `ReadResourceResponse` (`Complete(..)` via `.into()`); `ListToolsResult`/`ListResourcesResult` gained the SEP-2322 `result_type` and SEP-2549 `ttl_ms`/`cache_scope` fields, so struct literals no longer compile (`with_all_items(..)` instead); `ServerInfo` is deprecated for `ServerConfig`. Protocol version unchanged (`ProtocolVersion::LATEST` = `2025-11-25` in both).
- **#729 Merged**: opentelemetry family 0.27 → 0.32, clearing GHSA-w9wp-h8wv-79jx (medium, unbounded allocation in W3C Baggage propagation). `TracerProvider` → `SdkTracerProvider`; `Resource::new(vec![KeyValue::new("service.name", ..)])` became private → `Resource::builder().with_service_name(..).build()`; `tracing-opentelemetry` 0.28 → 0.33. Verified with `cargo test --features otlp --test observability_integration` (6 passed, incl. a real gRPC exporter build).

### Not fixed (non-actionable)
- **`lru` GHSA-rhfx-m35p-ff5j (low, patched 0.16.3)**: transitive via `quinn-proto 0.11.16` and `tantivy 0.22.1`, both on `lru ^0.12`. Not bumpable from this repo until those parents release; the alert stays open.
- **`libsql-sqlite3-parser` (low, no patched release)**: pinned by libsql upstream, already ignored in `deny.toml` and `dependabot.yml` with a documented reason.

### Final state
`main` at `c841d28`, zero open PRs, zero open issues; `cargo deny check advisories` ok and `cargo tree --locked --all-features` resolves (rmcp 3.4.0, opentelemetry_sdk 0.32.1). `plans/GOAP_STATE.md` metrics refreshed in place (`main_head`, `tests_count` 1092 → 1053 from the dedup waves, `integration_test_files` 70 → 71, `dependabot_alerts_open` 5 → 3). Local `target/` (116 GiB) reclaimed with `cargo clean`.

## 2026-09-15 (wave 2): ADR-0094 Persistence Owner Dedup Completed + PR Queue Landed

### Summary
Full `deduplicate_persistence_owner_bodies` wave (user-approved) plus the blocked PR queue. A freshly published advisory (RUSTSEC-2026-0285, rustls) had every open PR red on `Cargo Deny`; landing the one-file lockfile bump first, then `update-branch`-ing each PR so it inherited the fix, cleared the queue sequentially (#707, #708, #709, #710, #711 — all squash-merged with green CI, never `--auto`). Persistence now has a single implementation owner: root keeps only re-exports, one-line delegations and two infallible payload converters.

### Actions
- **#712 Merged** (`68d603d`): `cargo update -p rustls --precise 0.23.45` (+ `rustls-webpki` 0.103.15) for RUSTSEC-2026-0285; `cargo deny check advisories` and `cargo check --locked --all-features` green. Landed before the queue so every other PR could inherit it.
- **#709 commitlint fixed**: the two offending commits were rewound (`git reset --soft HEAD~2`) and recommitted as `docs: regenerate stale llms API listing` / `docs: use the 0.3 major-minor spec in install snippets`; the first body also had to be re-wrapped (a single 431-char line is parsed as a footer and trips `footer-max-line-length`).
- **#707/#708/#709/#710/#711 Merged**: dependabot action pin, triage record, repo hygiene (artifacts untracked + 0.3.8 version truth + llms), ADR-0071 rerank restoration, persistence phase 2. Each PR: `update-branch` → full CI → `gh pr merge --squash --delete-branch`.
- **Phase 3 (PR #713)**: `src/export_payload.rs` reduced from 177 to 51 lines — the payload/wire types and `unix_now_secs` now come from `csm-traits`, with `concept_to_export`/`export_to_concept` as the only root code; `ExportConcept` gained `#[serde(default)]` for legacy JSON. Verified: lib tests (195 on the pre-#711 base, 184 afterwards) + no-default-features (135), targeted integration suites, clippy/fmt, wasm32 check, a legacy-JSON import through the CLI, and a before/after binary comparison (field-level decode of both bincode payloads identical except the wall-clock field and pre-existing map ordering; cross-imports both ways).
- **Phase 4 (this PR)**: dead `record_concept_version` wrapper + its `#[allow(dead_code)]` removed from `csm-persistence`, the wasm stub export made mutually exclusive with the real one (PR #711 left `cargo clippy -p csm-persistence --all-features` failing with E0252 on a host target — CI only compiles default features for that crate, so nothing caught it), llms regenerated, CHANGELOG/GOAP/ACTIONS/plan-doc/PROGRESS/LEARNINGS updated; `queued_actions_count` 4 → 3, `action_last_completed: deduplicate_persistence_owner_bodies`.
- **Found, not fixed**: `WasmFramework::exportToBytes`/`importFromBytes` disagree on the bincode config (legacy vs `DefaultOptions`), so the browser round trip fails — pre-existing, documented in `progress/LEARNINGS.md` with the probe, left for its own change.

## 2026-09-15: PR Queue Clear — Draft Closed as No-Impact, Hygiene + Persistence Queue

### Summary
Zero open issues; two open PRs. Roasted both: #706 (Jules `perf(retrieval)`) closed as no-impact after allocator-level measurement, #707 (dependabot action SHA bump) kept as the single mergeable keeper. Session also surfaced three main-truth defects queued for the hygiene PR: the 0.3.8 version revert, tracked-but-gitignored `export.json`, and a 2.2 MB committed `cargo metadata` dump.

### Actions
- **#706 Closed** (no impact): outputs identical, allocations identical (13/13 allocs, 131,626 bytes map path; 12/12, 24,410 bytes fast path), timing deltas opposite in sign (−4.4% / +4.3%). Full roast comment posted to the PR; reopen path documented (attach criterion output from `benches/hybrid_benchmark.rs`).
- **#707 Keeper**: `pre-release-gate.yml` SHA pin `fa23953… v2.87.10` → `9534c84… v2.87.11`; CI green 27/27, `mergeStateStatus: CLEAN`, merge left to the maintainer.
- **Hygiene PR queued**: untrack `export.json` + `benchmarks/Cargo.lock`, remove `metadata.json`, align `migrations/007_add_hnsw_graph.sql` with the Rust schema, restore the 0.3.8 version/CHANGELOG bump.
- **Persistence queue (ADR-0094, user-approved)**: 4-phase `deduplicate_persistence_owner_bodies` — crate parity, root delegation, payload convergence, cleanup; executed after the hygiene and rerank PRs.
- **Rerank queue**: `src/framework_rerank.rs` (`probe_with_rerankers`, ADR-0071) is unreachable since #178 dropped its `mod` declaration and compiles clean — restore module + regression test.
- **Report**: `plans/PR_ROAST_2026_09_15.md`.

## 2026-09-11: Stale-Base Squash Revert Repair + PR Wave

### Summary
PR triage surfaced that the `943/fdcd069` squash sequence (Jules PR #683) was based on a stale fork and reverted already-merged main content: the gh-release v3.0.3 pin (#674), the uuid 1.26.0 lock entry (#677), the LSH `compute_hash_from_bytes` / pre-sized candidate-set work (#683, `a3eeaab`), the `progress/LEARNINGS.md` compaction, and `plans/PR_ROAST_2026_09_08.md`. Restored every reverted path in one dedicated PR and encoded the detection rule in `progress/LEARNINGS.md`.

### Actions
- **#686 Merged** (`ccbb7e8`): restores LSH bit-mask hashing + `HashSet::with_capacity(32)` (the #683 optimization clobbered by `9140aa4`).
- **#684 Queued**: `fix(singularity)` clamps `BridgeRetrieval::query` top_k to `MAX_TOP_K_LIMIT` (CWE-770), matching the BM25 clamp precedent.
- **Restore PR**: workflows back to `action-gh-release@efb3536… # v3.0.3`, `Cargo.lock` back to uuid 1.26.0 (`cargo metadata --locked` OK), `progress/LEARNINGS.md` + `progress/PROGRESS.md` + `plans/PR_ROAST_2026_09_08.md` restored from `f6117e7`, LSH comment corrections and `RetrievalConfig` struct literals restored from `a3eeaab`.
- **#685 Pending**: feature-gated `experimental-ils3d` hyperchaotic map (Sun et al. 2025); CI green, needs a naming-fidelity review before merge.
- **Report**: `plans/PR_ROAST_2026_09_11.md`.

---

## 2026-09-08: PR Roast — Sequential Merge of Remaining PR Wave

### Summary
Triaged all 7 open PRs (#679, #677, #676, #675, #674, #673, #660) per `pr-roast-triage`. Zero closures (no duplicates, no zero-impact PRs). Applied roast fixes: rewrote #660's stale body via REST API, marked draft #679 ready. Merging sequentially with full CI-truth verification per AGENTS.md (no `--auto`).

### Actions
- **#679 Merged** (`a73b8a0`): perf(retrieval) single-pass hash inserts in BFS expansion + fact dedup. Required `gh pr ready` (Jules draft) before merge.
- **#660 Body Fixed**: Stale re-export description replaced via `gh api -X PATCH`; follow-up queued to lift `MAX_TRAVERSAL_EXPANSIONS` into `GraphRagConfig`.
- **Branch Sync**: #660 synced via `gh api -X PUT .../pulls/660/update-branch` (no `gh pr update-branch` subcommand in installed gh); all 5 dependabot PRs queued for single-pass `@dependabot rebase` onto final main.
- **Report**: `plans/PR_ROAST_2026_09_08.md`.

---


## 2026-09-07: PR Roast Triage & Resolution of All Open Issues

### Summary
Comprehensive triage and roast of all 13 open PRs and 4 open GitHub issues. Closed duplicate PR #672 with no independent impact. Verified and merged PR #647, resolving and closing all 4 open issues (#639, #640, #641, #642). Deduplicated root retrieval modules, verified 100% Codacy pass, and established sequential manual merge plan for the remaining 11 open PRs.

### Actions
- **All 4 GitHub Issues Resolved**:
  - #639 (`RetrievalConfig abort knobs + for_token_count`): Implemented in `crates/csm-memory/src/singularity_retrieval.rs`.
  - #640 (`perf: cut probe_text / BridgeRetrieval work`): Parent issue resolved and closed.
  - #641 (`perf(bridge): cap expansion and Hamming only new IDs`): Implemented in `src/bridge_retrieval.rs` via `score_specific_candidates`.
  - #642 (`perf(retrieval): BM25 absence + stage-1 cap + HDC early-exit`): Implemented across retrieval pipeline.
- **PR #647 Merged** (`ff85763`): 28 CI checks green, 0 Codacy issues. Replaced ~970 lines of duplicate root retrieval files (`bm25.rs`, `rerank.rs`) with re-export shims over `csm_retrieval` (ADR-0094), reducing repository LOC by over 5,300 lines while keeping all files strictly under the 500 LOC gate.
- **PR #672 Closed**: Redundant duplicate of #653; stripped public wrapper, failed commitlint.
- **PR Titles Fixed via REST**: Updated #670 (`perf(core)`) and #660 (`fix(retrieval): cap GraphRAG traversal expansions at 1000 nodes`).
- **Codacy Gate Verified**: 0 issues across all 13 analyzed PRs.
- **Merge Plan Emitted**: `plans/PR_ROAST_2026_09_07.md`.

---

## 2026-07-27: PR Triage + CI Queue Fix

### Summary
GOAP-orchestrated sweep of open PRs. Closed #571 (fake perf optimization), merging #573 (dependabot).
Fixed release workflow timeout caused by CI queue starvation.

### Actions
- **PR #571 closed**: `perf(retrieval): optimize min/max loops` — replaced `.min()`/`.max()` with `<`/`>` comparisons. Net negative: removes docs, adds 4 mutation exclusions, reverses intentional design. LLVM generates identical code either way.
- **PR #573 merging**: dependabot bump `taiki-e/install-action` 2.83.4 → 2.85.2 (3 workflow files).
- **CI queue starvation**: Runs 30274366967 + 30275287578 stuck "queued" >1hr. Cancelled and re-triggered. Release run 30274367903 timed out at 1800s waiting for CI that never started.
- **Fix**: Enhanced `wait-for-ci` in release.yml to detect perpetual-queue and re-trigger CI.

---

## 2026-08-05: Review-to-merge sweep (PRs #597-#601)

### Summary
Reviewed and merged the final open PRs: #597 (direct SIMD hamming), #598 (graph candidate &str), #599 (native arm64 test job), plus state/docs PRs #600 and #601. The queue is now empty and every perf claim landed with same-machine criterion evidence attached.

### Actions
- **PR #597 merged** (`0b42edb`): `BHVec10240::hamming` dispatches directly over the packed `[u64; 160]` words — AVX2 (x86_64), NEON (aarch64), unrolled scalar fallback (wasm32) — eliminating the two `to_hvec()` layout conversions. Same-machine criterion ~2.6–2.75× faster (main 99.4/149.9 ns vs 37.7/54.5 ns). Codacy unsafe-usage findings fixed in code via `.codacy.yml` `exclude_paths` (repo policy); zero dashboard suppressions. Added forced-fallback unit test + lane-skip guard.
- **PR #599 merged** (`8d63b27`): `test-core-arm64` job on `ubuntu-24.04-arm` executes the NEON kernels in CI (previously compile-only on x86_64).
- **PR #598 merged** (`075cfe8`): `generate_graph_candidates` BFS borrows `&str` instead of cloning every candidate `String` — ~8% faster end-to-end (same-machine A/B: 238.3 → 218.5 µs, ~840 fewer allocs). Evidence bench committed: `benches/graph_candidates_benchmark.rs`.
- **PR #600/#601 merged** (`59c44d6`/`cb45951`): state records for all merged PRs; benchmark-methodology learnings (stale-binary detection, forced clean A/B, load-spike exclusion, deterministic test graphs); `benchmark-graph-candidates` CI regression gate (ceiling 600 µs, measured 258.9 µs on GitHub hardware).
- **Process notes**: commitlint footer-max-line-length requires every message line ≤ 100 chars; `npx commitlint | tail` swallows the exit code — always check `$?` explicitly.

---

## 2026-07-23: Wave 33 — CI Fixes + GOAP Orchestrator Hardening

- PR #551: WASM `--target nodejs` for CI + main concurrency guard
- PR #552: Removed dead `is_known_absent` (zero callers, zero tests)
- PR #553: GOAP orchestrator `status`/`wave`/`verify` commands + swarm patterns

---

## 2026-07-18: Framework Ops Perf + PR Triage

- PR #524-#526: namespace single-clone, parallel inject, split import locks
- Merged #528 (BM25 hot loop), #527 (Rayon probe_batch), #529 (hybrid partial top-k)
- Closed #520 (empty Jules research PR)

---

## 2026-05-23: Documentation Audit
Synchronized README, architecture docs, and books with implementation state.

## 2026-04-11: Persistence Field Integrity
Schema v6: `expires_at` + `canonical_concept_ids` across all persistence surfaces.

## 2026-04-09: Benchmark Harness (v0.3.1, v0.3.2)
BM25 Rayon parallelization (~40% speedup), p99/NDCG metrics, percentile fix.

## 2026-04-08: Hybrid Retrieval + v0.3.0
BM25+HDC fusion with query-length weights. Recall@1: 2.5% → 75%. Semantic Bridge (ADR-0061), table prefix (ADR-0063).

## 2026-04-06: Release Workflow
npm OIDC Trusted Publishing. Fixed duplicate CHANGELOG header breaking awk extraction.
