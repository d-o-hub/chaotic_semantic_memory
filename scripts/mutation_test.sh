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

TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="progress/mutation"
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
      # No bounded diff on a CI run — e.g. the Pre-Release Gate dispatched on main where
      # HEAD == origin/main, so the three-dot diff is empty. A full-tree pass here is not
      # viable: 1000+ mutants exceed the job's timeout-minutes and degrade into mass
      # build-timeouts (job 90089540259: ~1288 mutants, ~390 min estimate vs 120 min budget).
      # Mutation coverage is already enforced incrementally on every PR through the in-diff
      # path above, so skip cleanly instead of running an unbounded full-tree pass. Mirrors
      # the "no mutants in changed sources" skip below. Full-tree remains available via the
      # non-CI (local/nightly) branch and the `full` profile.
      echo "mutation fast: no diff against ${DIFF_TARGET} on a CI run; skipping full-tree fallback" >&2
      {
        echo "# Mutation Test Report"
        echo
        echo "- Timestamp (UTC): ${TIMESTAMP}"
        echo "- Profile: ${PROFILE}"
        echo "- Mutants: 0"
        echo "- Result: skip (no bounded diff on CI run; full-tree fallback disabled in CI)"
      } >"${REPORT_FILE}"
      echo "mutation score: no bounded diff on CI run, full-tree fallback skipped"
      exit 0
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

  # Unit tests only: integration suite dominates wall time (~8× unit suite).
  # We limit testing to only csm-retrieval and chaotic_semantic_memory to avoid
  # building irrelevant workspace packages, saving up to 80% build time.
  # TODO: expand this list of packages if new packages or workspace crates are added.
  TEST_ARGS+=(--lib -p csm-retrieval -p chaotic_semantic_memory)

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
    # const/eval mutants don't burn the full job budget (was 180s).
    FAST_ARGS+=(--timeout 120)
    FAST_ARGS+=(--minimum-test-timeout 15)
    FAST_ARGS+=(--build-timeout 150)
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

# Shared excludes: I/O, WASM stubs, MCP wire, uninteresting Drop/guards.
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
  --exclude-re "mcp::"
  --exclude-re "McpHandler::"
  --exclude "src/mcp/*"
  --exclude "src/persistence_wasm.rs"
  --exclude-re "replace > with >= in <impl Reranker for MmrReranker>::rerank"
  # CLI entry points: async I/O + side effects; --lib mutation cannot kill
  # "replace run_query -> Result<()> with Ok(())" without integration fixtures.
  --exclude-re "run_query"
  --exclude-re "replace run_query"
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
  # src/export_payload.rs. This file-level exclude also covers AbsenceEntry::normalize
  # (bridge_persistence.rs:264), so no separate mutant-label exclude is needed.
  --exclude "src/bridge_persistence.rs"
  # M1 BM25 absence short-circuit (ADR-0094 follow-up): short_circuit_if_known_absent
  # is a framework async path that requires a persisted absence store; its
  # Some/None branch is covered by tests/bm25_absence_short_circuit.rs, which the
  # --lib mutation profile does not run (same rationale as the framework probe
  # excludes above).
  --exclude-re "short_circuit_if_known_absent"
  # Test scaffolding: StubStore::list_absences is a trait method required by the
  # AbsenceStore stub but never called by the unit test, so mutating it to
  # Ok(vec![]) is unobservable.
  --exclude-re "StubStore>::list_absences"
)

# Preflight count (omit -j; listing does not need parallel workers).
if [[ "${PROFILE}" == "fast" ]] && [[ ${#FAST_ARGS[@]} -gt 0 ]]; then
  LIST_OUT="$(cargo mutants --list "${FAST_ARGS[@]}" "${EXCLUDE_ARGS[@]}" 2>/dev/null || true)"
  MUTANT_COUNT="$(printf '%s\n' "${LIST_OUT}" | grep -cE '\.rs:' || true)"
  echo "mutation preflight: ${MUTANT_COUNT:-0} mutants to test (jobs=${JOBS})" >&2
  if [[ "${MUTANT_COUNT:-0}" -eq 0 ]]; then
    echo "No mutants generated for changed files; writing empty report" | tee "${LOG_FILE}"
    {
      echo "# Mutation Test Report"
      echo
      echo "- Timestamp (UTC): ${TIMESTAMP}"
      echo "- Profile: ${PROFILE}"
      echo "- Mutants: 0"
      echo "- Result: skip (no mutants in diff)"
    } >"${REPORT_FILE}"
    if [[ "${CI_MODE}" == "true" ]]; then
      echo "mutation score: no mutants in changed Rust sources, CI check skipped"
      exit 0
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
RUSTFLAGS="" RUST_BACKTRACE=0 "${RUN_CMD[@]}" 2>&1 | tee "${LOG_FILE}"
RESULT="${PIPESTATUS[0]}"
set +o pipefail
set -e

{
  echo "# Mutation Test Report"
  echo
  echo "- Timestamp (UTC): ${TIMESTAMP}"
  echo "- Profile: ${PROFILE}"
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
    echo "mutation inventory: caught=${CAUGHT} missed=${MISSED} timeout=${TIMEOUTS} unviable=${UNVIABLE} unresolved=${UNRESOLVED}" >&2
    echo "mutation summary: total=${TOTAL} caught=${CAUGHT} timeout=${TIMEOUTS} missed=${MISSED} unviable=${UNVIABLE} score=${SCORE}%" >&2
  fi
  if [[ -z "${SCORE}" ]]; then
    SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
  fi

  # Timeout budget (ADR-0095): timeouts still count as "detected" for score, but
  # a high timeout rate fails the job so hangs cannot look like strong coverage.
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
    if grep -q -E 'No mutants generated|Diff changes no|No mutants to filter' "${LOG_FILE}" 2>/dev/null; then
      echo "mutation score: no Rust source files changed, CI check skipped"
      exit 0
    else
      echo "error: could not parse mutation score from ${LOG_FILE}" >&2
      exit 1
    fi
  elif awk -v s="${SCORE}" -v t="${THRESHOLD}" 'BEGIN { exit !(s >= t) }'; then
    echo "mutation score ${SCORE}% >= ${THRESHOLD}%, CI check passed"
    exit 0
  else
    echo "mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
    exit 1
  fi
fi

exit "${RESULT}"
