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
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

PROFILE="${1:-fast}"
shift || true

# Parallelism: CI sets MUTANTS_JOBS=4 (ubuntu-latest ≈ 4 vCPU); local default 1.
# Force JOBS=1 on fast profile to allow cargo to reuse build artifacts sequentially
# across mutants in a single target directory, avoiding parallel workspace rebuild timeouts.
if [[ "${PROFILE}" == "fast" ]]; then
  JOBS=1
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
    else
      echo "warning: no diff against ${DIFF_TARGET}; running full target set" >&2
    fi
  else
    echo "warning: --in-diff is unsupported by installed cargo-mutants; running full target set" >&2
  fi

  # Unit tests only: integration suite dominates wall time (~8× unit suite).
  # We limit testing to only csm-retrieval and chaotic_semantic_memory to avoid
  # building irrelevant workspace packages, saving up to 80% build time.
  TEST_ARGS+=(--lib -p csm-retrieval -p chaotic_semantic_memory)

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
    # Industry (Stryker et al.): detected = killed + timeout. Infinite-loop mutants
    # often only surface as timeouts. ADR-0095: also fail on a timeout *budget*
    # so a hung suite cannot masquerade as higher quality.
    EFFECTIVE_CAUGHT=$((CAUGHT + TIMEOUTS))
    if [[ "${VIABLE}" -gt 0 ]]; then
      SCORE="$(awk -v c="${EFFECTIVE_CAUGHT}" -v v="${VIABLE}" 'BEGIN { printf "%.4f", c*100/v }')"
    else
      SCORE="100"
    fi
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
