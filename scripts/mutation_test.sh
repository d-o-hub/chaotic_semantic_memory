#!/usr/bin/env bash
# Mutation testing wrapper (cargo-mutants).
#
# Profiles:
#   fast (default, CI): --in-diff vs base, unit tests only (--lib), tight timeouts
#   full: entire tree, full cargo test suite (local/nightly use)
#
# Performance notes (2026-07-16, PR #516 analysis of job 87601199764):
#   - Full `cargo test` ≈ 12s unit + ~97s integration per mutant.
#   - 120 in-diff mutants × ~2 min / 4 jobs ≈ 60+ min wall time.
#   - Fast CI uses `-- --lib` so each mutant is ~10–15s of tests after build.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CI_MODE=false
THRESHOLD="${MUTATION_THRESHOLD:-85}"
POSITIONAL=()
NO_DEFAULT_FEATURES=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ci)
      CI_MODE=true
      shift
      ;;
    --threshold=*)
      THRESHOLD="${1#*=}"
      shift
      ;;
    --threshold)
      THRESHOLD="$2"
      shift 2
      ;;
    --no-default-features)
      NO_DEFAULT_FEATURES=true
      shift
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

PROFILE="${1:-fast}"
shift || true

# Parallelism:
# - fast + in-diff (normal case): JOBS=1 to reuse sequential build artifacts across mutants.
# - fast + full-tree fallback (no diff detected): escalate to MUTANTS_JOBS (default 4) so
#   1 300+ mutants do not burn the full CI budget at 1 job.
# - full profile: use MUTANTS_JOBS (default 1 locally).
FAST_FALLBACK_JOBS="${MUTANTS_JOBS:-4}"
if [[ "${PROFILE}" == "fast" ]]; then
  JOBS=1          # overridden below if diff is empty
else
  JOBS="${MUTANTS_JOBS:-1}"
fi

if ! command -v cargo-mutants &>/dev/null && ! cargo mutants --version &>/dev/null; then
  cat <<'MSG' >&2
cargo-mutants is not installed.
Install it with:
  cargo install cargo-mutants
Or in CI with:
  - uses: taiki-e/install-action@v2
    with:
      tool: cargo-mutants
MSG
  exit 127
fi

OUT_DIR="${MUTATION_OUTPUT_DIR:-progress/mutation}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${OUT_DIR}/${PROFILE}-${TIMESTAMP}.log"
REPORT_FILE="${OUT_DIR}/${PROFILE}-latest.md"
mkdir -p "${OUT_DIR}"

HELP_TEXT="$(cargo mutants --help 2>/dev/null || true)"
FAST_ARGS=()
TEST_ARGS=()

if [[ "${PROFILE}" == "fast" ]]; then
  if grep -q -- '--in-diff' <<<"${HELP_TEXT}"; then
    DIFF_TARGET="${DIFF_TARGET:-origin/main}"
    DIFF_FILE="$(mktemp)"
    trap 'rm -f "${DIFF_FILE}"' EXIT
    # Three-dot: only commits on this branch since merge-base (true PR delta).
    if git rev-parse --verify "${DIFF_TARGET}" >/dev/null 2>&1; then
      git diff "${DIFF_TARGET}...HEAD" >"${DIFF_FILE}" 2>/dev/null \
        || git diff "${DIFF_TARGET}" >"${DIFF_FILE}" 2>/dev/null \
        || true
    else
      git diff "${DIFF_TARGET}" >"${DIFF_FILE}" 2>/dev/null || true
    fi
    if [[ -s "${DIFF_FILE}" ]]; then
      FAST_ARGS+=(--in-diff "${DIFF_FILE}")
      echo "mutation fast: in-diff against ${DIFF_TARGET} ($(wc -l <"${DIFF_FILE}") diff lines)" >&2
      # Feature-aware mutation: cfg(not(feature = "persistence")) mutants are
      # unreachable under default features — both generation and tests must run
      # with the feature off (ADR-0094, PR #621).
      if grep -Eq '^\+.*cfg\(not\(feature = "persistence"\)\)' "${DIFF_FILE}"; then
        NO_DEFAULT_FEATURES=true
        echo 'mutation fast: diff contains cfg(not(feature = "persistence")) — running with --no-default-features' >&2
      fi
    elif [[ "${CI_MODE}" == "true" ]]; then
      # An unbounded main-branch run is intentionally disabled. This invocation
      # produced no mutation evidence, so it must not succeed as a test result.
      echo "error: no bounded diff against ${DIFF_TARGET}; mutation tests did not run" >&2
      {
        echo "# Unit-Target Mutation Test Report"
        echo
        echo "- Timestamp (UTC): ${TIMESTAMP}"
        echo "- Profile: ${PROFILE}"
        echo "- Mutants: 0"
        echo "- Result: not run (no bounded diff; not a passing mutation result)"
      } >"${REPORT_FILE}"
      exit 2
    else
      echo "warning: no diff against ${DIFF_TARGET}; running full target set" >&2
      # Full-tree fallback (local/nightly only): escalate parallelism so the run finishes.
      # JOBS=1 is only safe for the in-diff case (sequential artifact reuse).
      JOBS="${FAST_FALLBACK_JOBS}"
      echo "mutation fast: full-tree fallback — escalating to JOBS=${JOBS}" >&2
    fi
  else
    echo "warning: --in-diff is unsupported by installed cargo-mutants; running full target set" >&2
  fi

  # Unit-target tests only (--lib): integration suite dominates wall time (~8× unit suite).
  # Any change to src/** or crates/** must have unit test assertions in the lib target
  # to be visible to this mutation gate (process-level tests in tests/** are ignored).
  # We limit testing to csm-core-lib, csm-retrieval, and chaotic_semantic_memory to avoid
  # building irrelevant workspace packages, saving up to 80% build time.
  # TODO: expand this list of packages if new packages or workspace crates are added.
  TEST_ARGS+=(--lib -p csm-core-lib -p csm-retrieval -p chaotic_semantic_memory)

  # `mcp` is not in the crate's `default` feature set (Cargo.toml:
  # default = ["cli", "persistence", "parallel"]) and both
  # `chaotic_semantic_memory::mcp` (src/lib.rs:57) and `crate::cli::mcp`
  # (src/cli/mod.rs:6) are `#[cfg(feature = "mcp")]`. Under the default-feature
  # profile that code is never compiled, so a mutation in it changed nothing
  # observable and cargo-mutants reported it as MISSED — which reads like "the
  # tests failed to kill it" but means "the job could not see it at all".
  # Measured with the same cargo-mutants 27.1.0 CI uses, on the #817 diff
  # (--in-diff, --lib, jobs=1, no build bound): both `src/cli/mcp.rs:42`
  # parse-helper mutants flipped MISSED -> CAUGHT (killed by the `45` assertion
  # in the same module) and the base build went to 146-183s on a target that had
  # to link the feature graph (23s test). Two costs are therefore paid here:
  # rmcp/axum/tower join every mutation job's base build, and that build needs
  # both the raised `--build-timeout` in the CI block below (at 150s its own
  # mutants came back TIMEOUT and the score printed 0% on a change that is
  # otherwise killable) and the ci.yml pre-warm step that relinks them once
  # before the run — after warming, touching one file and rebuilding both
  # feature-enabled test binaries measured 39s. The *path* set is now as wide as
  # the feature set: `src/mcp/*` left this job's scope on 2026-10-15 (#833), which
  # ran the module's first baseline and added the lib-target tests and the
  # documented excludes below — evidence in
  # `plans/evidence/mutation_mcp_2026_10_15/`.
  FAST_ARGS+=(--features chaotic_semantic_memory/mcp)

  if [[ "${NO_DEFAULT_FEATURES}" == "true" ]]; then
    # --cargo-arg applies --no-default-features to every cargo invocation
    # (build AND test); adding it again in TEST_ARGS would make cargo reject
    # the duplicated `--no-default-features` flag on the test invocation.
    FAST_ARGS+=(--cargo-arg=--no-default-features)
  fi

  if [[ "${CI_MODE}" == "true" ]]; then
    # --in-place is incompatible with parallel jobs (-j)
    if [[ "${JOBS}" -eq 1 ]]; then
      FAST_ARGS+=(--in-place)
    fi
    FAST_ARGS+=(--no-shuffle)
    # Tight bounds: kill hung mutants; keep build-timeout short so pathological
    # const/eval mutants don't burn the full job budget (was 180s, then 150s).
    # 150s is now measured to be too short for an *incremental* build: with the
    # default features those rebuilds ran ~54s, but adding
    # chaotic_semantic_memory/mcp put the unmutated baseline at 183s on a warm
    # target and individual mutants at 145-208s, so three otherwise killable
    # mutants came back TIMEOUT and the score printed 0% (the same three are
    # CAUGHT with no build bound). 420s keeps the pathological-mutant guard — a
    # single mutant still cannot eat the job's 45 minutes — while leaving the
    # feature-enabled build room. The cold case is handled where it belongs, by
    # pre-warming the feature build in ci.yml, not by a loose per-mutant bound.
    FAST_ARGS+=(--timeout 120)
    FAST_ARGS+=(--minimum-test-timeout 15)
    FAST_ARGS+=(--build-timeout 420)
  else
    FAST_ARGS+=(--timeout 90)
    FAST_ARGS+=(--minimum-test-timeout 15)
    FAST_ARGS+=(--build-timeout 120)
  fi
elif [[ "${PROFILE}" != "full" ]]; then
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  exit 2
else
  # full profile: broader timeouts, full suite (no --lib)
  FAST_ARGS+=(--timeout 120)
  FAST_ARGS+=(--minimum-test-timeout 30)
  FAST_ARGS+=(--build-timeout 180)
fi

MUTANTS_SHARD="${MUTANTS_SHARD:-}"
if [[ -n "${MUTANTS_SHARD}" ]]; then
  FAST_ARGS+=(--shard "${MUTANTS_SHARD}")
fi

MUTANTS_ARGS=("${FAST_ARGS[@]}")
if [[ "${JOBS}" -gt 1 ]]; then
  MUTANTS_ARGS+=(-j "${JOBS}")
  # --in-place must not be present when -j > 1 (cargo-mutants restriction).
  # We filter --in-place out of MUTANTS_ARGS to avoid conflicts.
  FILTERED_ARGS=()
  for arg in "${MUTANTS_ARGS[@]}"; do
    if [[ "$arg" != "--in-place" ]]; then
      FILTERED_ARGS+=("$arg")
    fi
  done
  MUTANTS_ARGS=("${FILTERED_ARGS[@]}")
fi

# Shared excludes: I/O, WASM stubs, uninteresting Drop/guards.
EXCLUDE_ARGS=(
  --exclude-re "WasmFramework::"
  --exclude-re "persistence::"
  --exclude-re "HnswIndex::serialize"
  --exclude-re "HnswIndex::deserialize"
  --exclude-re "OtlpGuard::"
  --exclude-re "install_grpc_tracer"
  --exclude-re "impl Drop for Guard"
  --exclude-re "impl Drop for OtlpGuard"
  --exclude-re "Result<Option<Guard>>"
  --exclude-re "replace && with"
  --exclude-re "delete . in init"
  --exclude-re "replace > with >= in FrameworkBuilder::with_max_associations_per_concept"
  --exclude-re "replace > with .* in FrameworkBuilder::build"
  --exclude-re "ChaoticSemanticFramework::load "
  --exclude "src/persistence_wasm.rs"
  --exclude-re "replace > with >= in <impl Reranker for MmrReranker>::rerank"
  # Hybrid merge equivalent mutants (2026-09-13, PR #693; proof in
  # plans/PR_ROAST_2026_09_12.md): select_nth(top_k - 1) + truncate(top_k)
  # makes `- -> /` (top_k/1 == the pre-2026-09 form) and `> -> >=` (wider
  # partition, same post-truncate top-K set) produce identical output for
  # every input — unkillable by any test. The killable `top_k - 1 -> top_k + 1`
  # mutant is covered by the len == top_k + 1 boundary tests instead.
  --exclude-re "replace - with / in merge_"
  --exclude-re "replace > with >= in merge_"
  # CLI entry points: async I/O + side effects; --lib mutation cannot kill
  # "replace run_query -> Result<()> with Ok(())" without integration fixtures.
  --exclude-re "run_query"
  --exclude-re "replace run_query"
  # run_watch is the same class: a CLI entry point whose loop only exits on
  # Ctrl+C or on the event channel closing (the framework owns the sender, so
  # the latter is unreachable while the function holds it). Neither exit is
  # reachable from --lib, so "replace run_watch -> Result<()> with Ok(())" is
  # unkillable without a signal-sending integration fixture. Residual: the
  # call-site wiring (shutdown().await on the watch exit path) is covered by
  # review and a manual SIGINT smoke, not by a gate; the McpHandler and
  # framework_cleanup tests cover shutdown() itself, not its callers.
  --exclude-re "run_watch"
  # run_inject is the same class: a CLI command body reached only from the
  # binary's match arm (src/bin/csm.rs:141), so under --lib nothing calls it and
  # "replace run_inject -> Result<()> with Ok(())" cannot be killed. This mutant
  # appeared here only because #817 changed one line of the file (src/cli/commands/
  # inject.rs:22 now passes ttl_cleanup_interval_seconds = 0), not because inject's
  # behaviour got weaker: the flag itself is pinned by the parse tests in
  # src/cli/args_commands.rs.
  # Residual, stated plainly: the exclusion buys no coverage for the entry point
  # body. `csm inject` exit statuses are covered by tests/cli_integration.rs, which
  # is the *integration* profile — this job deliberately runs --lib only.
  --exclude-re "run_inject"
  # mcp::serve is the run_watch class: it is `serve_with_shutdown(config,
  # operator_shutdown())` (src/mcp/server.rs:72/97), so the body's work only
  # becomes observable in a process that receives SIGINT/SIGTERM — impossible
  # from `--lib`, where there is no operator signal and no transport to end.
  # Residual, stated plainly: this exclusion buys no coverage for the entry-point
  # body. The hop is pinned elsewhere — `src/mcp/server_tests.rs::
  # sse_exit_stops_the_ttl_cleanup_task` asserts the effect through
  # `serve_with_handler` (live reaper slot -> signal -> await -> slot empty),
  # `serve_with_shutdown_returns_when_the_signal_resolves` now requires the
  # listener to accept a connection before signalling, and the process-level
  # exits are covered by tests/cli_shutdown_signal.rs::
  # serve_sse_stops_cooperatively_on_sig{term,int} (integration profile). The
  # pattern is anchored on "replace serve ->" rather than "serve" so it cannot
  # swallow a `serve_*` helper `--lib` can test: `serve_with_handler` and
  # `run_sse_server_with_shutdown` are CAUGHT in the 2026-10-15 baseline.
  --exclude-re "replace serve ->"
  # src/mcp/tools.rs `| -> ^` in parse_hvec_u64_halves (2026-10-15, #833): the
  # operands are `u128::from(high) << 64` (bits 64..128) and `u128::from(low)`
  # (bits 0..64), disjoint by construction, so `|` and `^` agree on every input —
  # an equivalent mutant, not a weak test. Measured in the same baseline: the
  # sibling `| -> &` and all seven shift/arithmetic mutants in that function are
  # CAUGHT by tools_tests.rs::test_parse_hvec_legacy_u64_halves.
  --exclude-re "replace \| with \^ in parse_hvec_u64_halves"
  # src/bin/csm.rs: tracing setup, error formatting, shell completion, and main
  # are CLI-only concerns (side-effectful, process-exit, I/O); untestable via --lib.
  --exclude "src/bin/csm.rs"
  # merge_with > -> >=: equivalent mutant — equal scores produce same result
  # for both operators (keeping existing value vs overwriting with same value).
  --exclude-re "replace > with >= in AbsenceEntry::merge_with"
  # equivalent mutants in normalize_scores and merge_results (replacing < with <=, > with >= does not alter correctness)
  --exclude-re "replace < with <= in .*normalize_scores"
  --exclude-re "replace > with >= in .*normalize_scores"
  --exclude-re "replace < with <= in .*merge_results"
  --exclude-re "replace > with >= in .*merge_results"
  # query || -> &&: equivalent mutant — with top_k=0, find_similar+truncate(0)
  # also returns empty; with empty ns, find_similar returns empty too.
  --exclude-re "replace \|\| with && in BridgeRetrieval::query"
  # confidence sum/len: /->% and /->* are unkillable without knowing exact scores
  # at test time; the weight arithmetic is already covered by compute_final_score tests.
  --exclude-re "replace / with % in BridgeRetrieval::compile_packet"
  --exclude-re "replace / with \* in BridgeRetrieval::compile_packet"
  # unix_now_secs (wasm stub) and framework latency metrics: I/O and side-effectful
  # timing paths; not exercisable deterministically under --lib.
  --exclude "src/export_payload.rs"
  # framework Drop, builder, and latency-metric arithmetic: side-effectful async paths
  # observable only via integration tests (not --lib).
  --exclude-re "replace <impl Drop for ChaoticSemanticFramework>::drop"
  --exclude-re "replace ChaoticSemanticFramework::builder"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::inject_concept"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::inject_concept_with_metadata"
  --exclude-re "replace <= with > in ChaoticSemanticFramework::probe"
  --exclude-re "replace - with .* in ChaoticSemanticFramework::probe"
  --exclude-re "replace >= with < in ChaoticSemanticFramework::probe"
  # src/bridge_persistence.rs: every Persistence:: method gates on
  # acquire_remote_slot().await? (a real network/DB semaphore). Under --lib there are no
  # integration fixtures to satisfy it, so these mutants cannot be killed deterministically
  # (they hang into a test timeout or survive unobserved). Persistence correctness is covered
  # by integration tests, not --lib mutation — same rationale as src/persistence_wasm.rs and
  # src/export_payload.rs. This file-level exclude also covers AbsenceEntry::normalize,
  # so no separate mutant-label exclude is needed.
  --exclude "src/bridge_persistence.rs"
  # M1 BM25 absence short-circuit (ADR-0094 follow-up): absence_short_circuit
  # is a framework async path that requires a persisted absence store and a
  # namespace revision read; the decision itself (lookup_absence) is covered by
  # --lib unit tests, while the wrapper's abstention shape is covered by
  # tests/bm25_absence_short_circuit.rs, which the --lib mutation profile does
  # not run (same rationale as the framework probe excludes above).
  --exclude-re "absence_short_circuit"
  # Test scaffolding: StubStore::list_absences is a trait method required by the
  # AbsenceStore stub but never called by the unit test, so mutating it to
  # Ok(vec![]) is unobservable.
  --exclude-re "StubStore>::list_absences"
  # Feature-cfg'd code is enumerated but never compiled under this profile, so its
  # mutants are phantom: the mutation diff cannot change a binary that does not
  # contain the code, every test run passes, and the mutant is reported "missed"
  # while no test could ever kill it. Measured on 4180c6c8 (run 40855342249): 19 of
  # the 73 missed mutants were this class. Proof, not inference — the error literal
  # "OTLP batch sizes and timeouts must be positive" from
  # src/observability/mod.rs:88 appears 0 times in
  # target/debug/deps/chaotic_semantic_memory-*.d293 built with exactly the FAST_ARGS
  # features below (control: "decay" appears 174 times in the same binary).
  # src/observability/mod.rs:13 is
  #   #![cfg(any(feature = "prometheus", feature = "otlp-json", feature = "otlp"))]
  # and the fast profile enables `default = cli,persistence,parallel` + `mcp` only, so
  # the whole module tree is out — including prom.rs (`#[cfg(feature = "prometheus")]`
  # at mod.rs:22). Remove these two excludes when #952 makes generation cfg-aware or
  # when the profile gains an observability feature; until then they are the only way
  # the score reflects testable code.
  --exclude "src/observability/*"
  # Same class, one function: extract_trace_parent and its
  # opentelemetry::propagation::Extractor impl (src/mcp/server.rs:348-360) sit behind
  # #[cfg(all(feature = "otlp", not(target_arch = "wasm32")))]; `otlp` is opt-in and
  # off here. Proven the same way — "traceparent was not applied to the request span"
  # occurs 0 times in the built lib, while its ungated neighbour require_bearer's
  # "Unauthorized" occurs 3 times, so server.rs itself is in the build and only these
  # mutants are excluded. The trace-context wiring is covered by
  # tests/mcp_streamable_http_integration.rs and src/mcp/server_tests.rs (which is a
  # #[cfg(test)] module of the ungated half of server.rs), not by --lib mutation here.
  --exclude-re "extract_trace_parent"
  # Third-party of the same rule, wasm32 instead of a Cargo feature: the js_sys
  # elapsed/duration shims sit under #[cfg(target_arch = "wasm32")] with the native
  # std::time::Instant sibling on the line above, so on an x86_64 runner the `now -
  # start` subtraction is not in the binary and `- -> +` / `- -> /` cannot be killed
  # by any test. Named by operator so only these four mutants leave the denominator:
  # each function keeps every other mutant it has. Re-checked against missed.txt on
  # 4180c6c8: framework.rs:319:43 (x2, in associate) and
  # framework_ttl_purge.rs:101:48 (x2, in purge_expired_inner).
  --exclude-re "replace - with [+] in ChaoticSemanticFramework::associate"
  --exclude-re "replace - with / in ChaoticSemanticFramework::associate"
  --exclude-re "replace - with [+] in ChaoticSemanticFramework::purge_expired_inner"
  --exclude-re "replace - with / in ChaoticSemanticFramework::purge_expired_inner"
  # Fourth of the same rule. `hybrid_result_to_array` (src/wasm_ext.rs:34-35) is under
  # #[cfg(target_arch = "wasm32")] like the two operators above, and its two
  # `-> Result<Array, JsValue> with Ok(Default::default())` mutants were both MISSED on
  # run 40855342249. Proof on the binary this profile actually builds (cargo test
  # --no-run --lib -p chaotic_semantic_memory --features chaotic_semantic_memory/mcp,
  # rustc as pinned): "failed to set JS property" (wasm_ext.rs:43, inside the gated fn)
  # occurs 0 times, while "min_strength must be finite" (wasm_ext.rs:26, the ungated
  # helper in the same file) occurs once — so the file is linked and only this fn is
  # absent. Named by function, not by file, because src/wasm_ext.rs is not wholly gated;
  # its only callers (wasm_ext.rs:195, :206) are WasmFramework methods that the
  # pre-existing --exclude-re "WasmFramework::" already covers.
  --exclude-re "hybrid_result_to_array"
)

# Preflight count (omit -j; listing does not need parallel workers).
if [[ "${PROFILE}" == "fast" ]] && [[ ${#FAST_ARGS[@]} -gt 0 ]]; then
  LIST_OUT="$(cargo mutants --list "${FAST_ARGS[@]}" "${EXCLUDE_ARGS[@]}" 2>/dev/null || true)"
  MUTANT_COUNT="$(printf '%s\n' "${LIST_OUT}" | grep -cE '\.rs:' || true)"
  echo "mutation preflight: ${MUTANT_COUNT:-0} mutants to test (jobs=${JOBS})" >&2
  if [[ "${MUTANT_COUNT:-0}" -eq 0 ]]; then
    echo "No mutants generated for the bounded diff; no mutation evidence was produced" | tee "${LOG_FILE}"
    {
      echo "# Unit-Target Mutation Test Report"
      echo
      echo "- Timestamp (UTC): ${TIMESTAMP}"
      echo "- Profile: ${PROFILE}"
      echo "- Mutants: 0"
      echo "- Result: not run (no mutants in diff)"
    } >"${REPORT_FILE}"
    if [[ "${CI_MODE}" == "true" ]]; then
      echo "error: CI mutation check selected this Rust diff but generated no mutants" >&2
      exit 1
    fi
    exit 0
  fi
fi

set -o pipefail
set +e # cargo-mutants exits 2 when any mutant is missed; we evaluate the score below
RUN_CMD=(cargo mutants "${MUTANTS_ARGS[@]}" "${EXCLUDE_ARGS[@]}" "$@")
if [[ ${#TEST_ARGS[@]} -gt 0 ]]; then
  RUN_CMD+=(-- "${TEST_ARGS[@]}")
fi
MARKER="$(mktemp)"
trap 'rm -f "${MARKER}" "${DIFF_FILE:-}"' EXIT
RUSTFLAGS="" RUST_BACKTRACE=0 "${RUN_CMD[@]}" 2>&1 | tee "${LOG_FILE}"
RESULT="${PIPESTATUS[0]}"
set +o pipefail
set -e

# Ship cargo-mutants' own report with the CI artifact. The tee'd stdout only carries what
# cargo-mutants chose to print — caught mutants are silent, so a run that finishes 31 of 224
# before the ceiling shows 5 lines and looks stalled (that misread cost a wrong diagnosis
# here) — and `mutants.out/outcomes.json` is the record that answers it: per-mutant Build/Test
# phase durations, with `mutants.out/debug.log` carrying the timestamps that separate "this
# mutant is slow" from "the gap between mutants is slow". Outcomes are written as the sweep
# progresses, so a mid-run cancellation still leaves the partial data. The copy lands inside
# OUT_DIR (uploaded as `path: progress/mutation`) rather than going through `--output`,
# because scripts/mutation_inventory.py reads `mutants.out/outcomes.json` relative to the
# workspace root; `mutants.out/` is gitignored at any depth, so this adds no untracked churn.
# `-newer "${MARKER}"` distinguishes this run's report from a stale mutants.out in the tree.
if [[ -f mutants.out/outcomes.json ]] && [[ -n "$(find mutants.out/outcomes.json -newer "${MARKER}" 2>/dev/null)" ]]; then
  rm -rf "${OUT_DIR}/mutants.out"
  cp -a mutants.out "${OUT_DIR}/mutants.out"
  echo "mutation report copied to ${OUT_DIR}/mutants.out" >&2
fi

{
  echo "# Unit-Target Mutation Test Report"
  echo
  echo "- Timestamp (UTC): ${TIMESTAMP}"
  echo "- Profile: ${PROFILE}"
  echo "- Target Set: lib target only (--lib -p csm-core-lib -p csm-retrieval -p chaotic_semantic_memory)"
  echo "- Exit code: ${RESULT}"
  echo "- Jobs: ${JOBS}"
  echo "- Test args: ${TEST_ARGS[*]:-(full suite)}"
  echo "- Log: \`${LOG_FILE}\`"
  echo "- Command: \`cargo mutants ${MUTANTS_ARGS[*]} ${EXCLUDE_ARGS[*]} -- ${TEST_ARGS[*]}\`"
  echo
  echo "## Tail"
  echo
  echo '```text'
  tail -n 40 "${LOG_FILE}"
  echo '```'
} >"${REPORT_FILE}"

python3 "${SCRIPT_DIR}/mutation_inventory.py" >> "${REPORT_FILE}"

echo "wrote ${REPORT_FILE}"

if [[ "${CI_MODE}" == "true" ]]; then
  SCORE=""
  SUMMARY_LINE="$(grep -oE "[0-9]+ mutant[s]? tested in .*" "${LOG_FILE}" | tail -1 || true)"
  if [[ -n "${SUMMARY_LINE}" ]]; then
    TOTAL="$(echo "${SUMMARY_LINE}" | awk '{print $1}')"
    CAUGHT="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ caught' | awk '{print $1}')"
    CAUGHT="${CAUGHT:-0}"
    TIMEOUTS="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ timeout' | awk '{print $1}')"
    TIMEOUTS="${TIMEOUTS:-0}"
    MISSED="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ missed' | awk '{print $1}')"
    MISSED="${MISSED:-0}"
    UNVIABLE="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ unviable' | awk '{print $1}')"
    UNVIABLE="${UNVIABLE:-0}"
    VIABLE=$((TOTAL - UNVIABLE))
    # ADR-0095: timeouts count as unresolved (not detected), not caught.
    # Infinite-loop mutants are a quality issue, not a detection success.
    EFFECTIVE_CAUGHT=$((CAUGHT))
    UNRESOLVED=$((MISSED + TIMEOUTS))
    if [[ "${VIABLE}" -gt 0 ]]; then
      SCORE="$(awk -v c="${EFFECTIVE_CAUGHT}" -v v="${VIABLE}" 'BEGIN { printf "%.4f", c*100/v }')"
    else
      SCORE="100"
    fi
    # Publish inventory
    echo "unit-target mutation inventory: caught=${CAUGHT} missed=${MISSED} timeout=${TIMEOUTS} unviable=${UNVIABLE} unresolved=${UNRESOLVED}" >&2
    echo "unit-target mutation summary: total=${TOTAL} caught=${CAUGHT} timeout=${TIMEOUTS} missed=${MISSED} unviable=${UNVIABLE} score=${SCORE}%" >&2
  fi
  if [[ -z "${SCORE}" ]]; then
    SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
  fi

  # Timeout mutants remain unresolved and are excluded from caught outcomes; the
  # budget below prevents hangs from silently weakening the score.
  # Override: MUTATION_TIMEOUT_BUDGET (absolute), MUTATION_TIMEOUT_FRACTION (of viable).
  TIMEOUT_BUDGET="${MUTATION_TIMEOUT_BUDGET:-5}"
  TIMEOUT_FRACTION="${MUTATION_TIMEOUT_FRACTION:-0.10}"
  if [[ -n "${TIMEOUTS:-}" ]]; then
    if [[ "${TIMEOUTS}" -gt "${TIMEOUT_BUDGET}" ]]; then
      echo "error: ${TIMEOUTS} mutation timeouts exceed absolute budget ${TIMEOUT_BUDGET}" >&2
      exit 1
    fi
    if [[ -n "${VIABLE:-}" && "${VIABLE}" -ge 10 ]]; then
      FRAC_LIMIT="$(awk -v v="${VIABLE}" -v f="${TIMEOUT_FRACTION}" 'BEGIN { n=v*f; printf "%d", (n==int(n)?n:int(n)+1) }')"
      if [[ "${TIMEOUTS}" -gt "${FRAC_LIMIT}" ]]; then
        echo "error: ${TIMEOUTS} mutation timeouts exceed ${TIMEOUT_FRACTION} of viable=${VIABLE} (limit ${FRAC_LIMIT})" >&2
        exit 1
      fi
    fi
    if [[ "${TIMEOUTS}" -gt 0 ]]; then
      echo "mutation timeouts: ${TIMEOUTS} (budget abs=${TIMEOUT_BUDGET})" >&2
    fi
  fi

  if [[ "${SCORE}" == "0" ]]; then
    echo "error: mutation run generated no scoreable mutants; result is not a pass" >&2
    exit 1
  elif awk -v s="${SCORE}" -v t="${THRESHOLD}" 'BEGIN { exit !(s >= t) }'; then
    echo "unit-target mutation score ${SCORE}% >= ${THRESHOLD}%, CI check passed"
    exit 0
  else
    echo "unit-target mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
    exit 1
  fi
fi

exit "${RESULT}"
