# PROGRESS

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
