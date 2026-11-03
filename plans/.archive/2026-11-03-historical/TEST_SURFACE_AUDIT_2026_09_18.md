# Test & source surface audit — 2026-09-18

Executes the last queued GOAP action `deduplicate_test_and_source_surfaces`
(ADR-0094 + ADR-0095). The point of the action is a *behavior-based* test
inventory, so the audit had to decide, per cluster, whether two tests assert
the same behavior — not whether they look similar.

## Method

1. Three read-only scouts mapped the surfaces: root `tests/*.rs` (71 files,
   538 tests) against crate-side tests; root `src/**` inline modules against
   the owner crates; and the available coverage tooling.
2. Every proposed deletion was then re-checked mechanically:
   `/tmp`-only script comparing *normalized test bodies* (comments stripped,
   whitespace collapsed) between the candidate and its claimed survivor, so a
   rename or a reformat cannot hide a duplicate — and a different literal
   cannot fake one.
3. Anything that still did not match byte-wise was read by hand and judged on
   the behavior asserted (setup, inputs, assertion strength, API surface).

**A duplicate is only a duplicate if the surviving test asserts the same or a
strictly stronger behavior.** Same name is a lead, not proof.

## What was *not* a duplicate (the action's premise corrected)

Six root integration files were proposed for deletion as "verbatim
duplicates". They are not:

| Candidate | Claimed survivor | Why it must stay |
|---|---|---|
| `tests/cache_lru_coverage.rs` | `tests/cache_lru.rs` | asserts `metrics.cache_evictions_total >= 1` and exact cache-hit deltas through `probe_batch_cached`; the survivor asserts result membership through `probe` and never reads metrics. Different API, different assertions. |
| `tests/ttl_lifecycle.rs` | `tests/framework_ttl_integration.rs` | survivor asserts the stored `expires_at` window; the candidate asserts the concept is *retrievable* after TTL injection (retrieval path). |
| `tests/builder_advanced.rs` | `tests/builder_config.rs` | 9 tests vs 5, including `*_invalid_fails` cases and `max_sequence_length`; mechanical body comparison found zero identical bodies. |
| `tests/path_validation_errors.rs` | `tests/path_validation.rs` | checks the error *variant* on a path-too-long, traversal, absolute-path-outside-allowed set; survivor checks the accept/reject edges. |
| `tests/critical_error_paths.rs` | `tests/validation_errors.rs` | error-path assertions across reservoir/metadata/probe limits; zero identical bodies. |
| `tests/batch_ops_coverage.rs` | `tests/batch_operations.rs` | some names repeat, bodies differ (batch/cache interaction). |

Deleting them would have dropped unique assertions (metrics accounting, error
variants, retrieval-after-TTL) while looking like a clean "duplicate" removal.
They stay; this table is the evidence that the root integration surface is
now behavior-distinct from the crate surfaces.

## What *was* duplicated, and was removed

| Removed | Tests | Canonical owner |
|---|---|---|
| `src/embedding/mod.rs` test module | 12 | `crates/csm-embedding/src/lib.rs` (8 byte-identical, 4 identical after the env-lock port below) |
| `src/persistence_wasm.rs` test module | 6 | `crates/csm-persistence/src/persistence_wasm.rs` (the root file is `#[cfg(target_arch = "wasm32")]` only, and its test module referenced an undefined helper + missing `MemoryError` import — it could never compile) |
| `crates/csm-core-lib/src/maps/neural_circuit.rs` tests | 3 | `crates/csm-chaos/src/maps/neural_circuit.rs` |
| `src/lib.rs::encoder_lib_tests::encode_text_is_deterministic` | 1 | `crates/csm-core-lib/src/encoder.rs::encode_deterministic` |
| `src/export_payload/export_payload_tests.rs::test_unix_now_secs` | 1 | `crates/csm-traits/src/lib.rs::test_unix_now_secs` |
| `src/wasm_ext_tests.rs::hvec_bytes_roundtrip` | 1 | `crates/csm-core-lib/src/hyperdim_tests.rs::test_serialization` |
| `src/wasm_graph_rag.rs` empty `mod tests` | 0 | — |

Kept deliberately (unique behavior despite similar names):
`src/lib.rs::encode_text_produces_nonzero_output` (documented mutation kill),
`encode_different_inputs_differ`,
`src/export_payload/export_payload_tests.rs` metadata conversion tests
(per-variant plus the invalid-number fallback, which the owner's roundtrip
test does not reach), `hvec_bytes_length`,
`hvec_from_bytes_invalid_length`.

## Source-surface dedup

`csm-core-lib::maps::neural_circuit` held a second copy of the CDNCM map
(`f64::tanh`, unconditional serde derives) while `csm-chaos` owns it
(`libm::tanh`, feature-gated serde) — and nothing enabled
`csm-core-lib/experimental-neural-circuit` (the root forwards the feature to
`csm-chaos` only), so the copy was unreachable.

It is now a re-export (`pub use csm_chaos::maps::neural_circuit::NeuralCircuitMap;`)
and the feature forwards to the owner:
`experimental-neural-circuit = ["dep:csm-chaos", "csm-chaos/experimental-neural-circuit", "csm-chaos/std", "csm-chaos/serde"]`,
so downstream users keep both the API and the serde capability. Trajectories
now come from the owner's `libm::tanh` implementation.

Checked in the same pass and kept: `csm-core-lib::hashing::chaotic_lsh` is a
documented backward-compatible *wrapper* (re-export + thin type), not a second
implementation, and its test exercises the wrapper. Same for the other
`pub use csm_*` shims in `src/**`.

## Fix ported: env-mutating tests in the owner crate

The root copies of the embedding tests carried the `ENV_TEST_LOCK` guard from
PR #700; `crates/csm-embedding` still had the racy pattern with the false
"single-threaded test is sound" SAFETY comment on 4 tests
(`get_provider_{openai,voyage}_{with_feature,with_model}_returns_provider`).
The guard was ported to the crate (4 guards, 8 env mutations) *before*
deleting the root copies, so the deletion cannot regress the race fix.

## Coverage methodology (ADR-0095)

`scripts/update-coverage.sh` measured a test-LOC/source-LOC ratio, ignored
`crates/` entirely, and rewrote README rows that no longer exist; nothing ran
it except the pre-commit hook. Replaced by `scripts/coverage-report.sh`:

- `inventory` (fast, no test execution) — unique compiled behavior per layer,
  plus same-name leads across files.
- `llvm-cov` — line + branch coverage for the workspace lib targets
  (`cargo +nightly llvm-cov --workspace --exclude benchmarks --exclude
  csm-duckdb --lib --branch`; branch coverage is nightly-only, the script
  falls back to line coverage on other toolchains).

### Measured coverage (nightly, `--branch`)

| Targets | Lines | Branches | Functions |
|---|---|---|---|
| `--lib` only (unit tests, fast) | 8 082/11 818 = 68.39 % | 655/1 232 = 53.17 % | 904/1 552 = 58.25 % |
| `--lib --tests` (unit + all 71 integration binaries) | 10 168/13 680 = **74.33 %** | 917/1 431 = **64.08 %** | 1 200/1 768 = 67.87 % |

The unit-only number understates behavior coverage by 6 points of lines and 11
points of branches, because 538 of the 1 032 test functions live in `tests/`;
that is why the script defaults to the fast mode but ships the full one.
Artifacts: `target/coverage/coverage.json`.

Touched modules: `csm-embedding` 94.00 % lines; `persistence_retry` 75.00 %
lines / 33.33 % branches (its retry paths are exercised by the integration
test in `tests/persistence_concurrency.rs`).

### Unique compiled behavior (inventory)

| Layer | Files | Tests |
|---|---|---|
| `tests/` (root integration) | 71 | 538 |
| `src/` (root facade) | 24 | 175 |
| `crates/*/src` (owners) | 55 | 296 |
| `crates/*/tests` | 3 | 20 |
| **Total** | | **1029** |

Before the dedup: 1053 (root facade 196, owner crates 299). The 24 removals
are exactly the duplicate bodies listed above; the action's July count in
`GOAP_AUDIT_2026_07_14.md` (1 034) pre-dates the waves that already removed the
root hybrid/BM25 test copies.

Same-name leads remaining: 8 pairs, each checked and kept (after the neural
circuit dedup removed three of them; the `chaotic_lsh` pair is facade↔owner).

## Verification

- Baseline `cargo test --all-features` before the deletions: **exit 0**.
- After the deletions, re-export and env-lock port: `cargo test --all-features`
  **exit 0**; `cargo test -p csm-embedding --all-features` 15 passed;
  `cargo test -p csm-core-lib --features experimental-neural-circuit --lib`
  75 passed; `cargo test -p csm-chaos --all-features --lib` 20 passed.
- `cargo clippy --all-targets --all-features` clean, `cargo fmt --all --check`
  clean.
- No behavior deletion found in review: every removed body has a byte-identical
  or strictly stronger counterpart at the owner (table above).
