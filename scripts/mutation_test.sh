#!/usr/bin/env bash
# scripts/mutation_test.sh — cargo-mutants wrapper with fail-closed CI semantics.
#
# Usage:
#   scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]
#   scripts/mutation_test.sh --self-test
#
# Option flags:
#   --ci                hard-fail when the mutation score is below the threshold;
#                       also makes any empty/unscoped run a hard failure instead of
#                       a silent pass.
#   --threshold=N       minimum acceptable mutation score (default: $MUTATION_THRESHOLD or 85)
#   --self-test         run the embedded shell-based test battery and exit
#
# Exit codes (stable, CI-compatible — CI jobs only check zero/non-zero):
#   0    pass (score >= threshold), or a local empty-run notice, or self-test pass
#   1    fail-closed: score below threshold, unparseable result, or an in-diff scan
#        that yields no changed production files (src/**/*.rs, crates/**/*.rs).
#        Also used when cargo-mutants reports an empty run in --ci mode.
#   2    usage error (unknown profile, invalid --threshold)
#   127  cargo-mutants is not installed
#   Non-CI runs additionally propagate cargo-mutants' own exit code
#        (2 = missed mutants, 3 = timeouts, 4 = baseline failing, 6 = diff invalid).
#
# Machine-readable inventory (deterministic dir, refreshed per profile):
#   target/mutation-artifacts/<profile>/
#     summary.txt        key=value inventory incl. the CI-parsable MUTATION_SUMMARY line
#     caught.txt         mutant names killed by tests
#     missed.txt         mutants that survived (may hide behaviorally-equivalent mutants;
#                        document proven ones in scripts/mutation-equivalent.txt)
#     timeout.txt        mutants that timed out — UNRESOLVED, counted as missed
#     unviable.txt       mutants that did not compile — excluded from the denominator
#     equivalent.txt     copy of scripts/mutation-equivalent.txt (proven-equivalent set)
#     candidate-count.txt  mutants generated before filters (for the "excluded" count)
#
# The CI-parsable line is echoed to stdout AND written into summary.txt:
#   MUTATION_SUMMARY: profile=… exit=… total=… caught=… viable=… missed=…
#                     timeout=… unviable=… excluded=… score=… threshold=… result=PASS|FAIL|EMPTY
#
# Count semantics (exact):
#   total    = mutants actually tested (cargo-mutants "N mutants tested")
#              unviable mutants are included in total but NOT in viable.
#   viable   = total - unviable            (the score denominator)
#   caught   = mutants killed by tests
#   missed   = mutants that survived tests (equivalent mutants are a subset;
#              they can only be distinguished when proven and recorded)
#   timeout  = UNRESOLVED: the mutant ran long and was killed by the timeout. It
#              is not a kill and not proof of equivalence, so it counts toward
#              the missed bucket and lowers the score.
#   unviable = did not compile — excluded from the denominator (no signal)
#   excluded = generated candidates that were not tested at all
#              (removed by the --exclude filters and/or the --in-diff scope)
#   score    = caught * 100 / viable  (timeouts and missed both reduce it)
#
# Equivalent-vs-unviable classification: see scripts/mutation-equivalence.md.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ============================================================================
# Pure helpers — kept side-effect free so `--self-test` can assert on them.
# ============================================================================

# Parse a cargo-mutants summary line like
#   "42 mutants tested in 10.2s: 30 caught, 5 missed, 3 timeouts, 4 unviable"
# into "TOTAL CAUGHT MISSED TIMEOUT UNVIABLE" (5 fields, defaults 0).
mutation_parse_summary() {
  local line="$1"
  local total caught missed timeout unviable
  total="$(grep -oE '^[0-9]+' <<<"${line}" || true)"
  total="${total:-0}"
  caught="$(grep -oE '[0-9]+ caught' <<<"${line}" | awk '{print $1}' | tail -1 || true)"
  caught="${caught:-0}"
  missed="$(grep -oE '[0-9]+ missed' <<<"${line}" | awk '{print $1}' | tail -1 || true)"
  missed="${missed:-0}"
  timeout="$(grep -oE '[0-9]+ timeouts?' <<<"${line}" | awk '{print $1}' | tail -1 || true)"
  timeout="${timeout:-0}"
  unviable="$(grep -oE '[0-9]+ unviable' <<<"${line}" | awk '{print $1}' | tail -1 || true)"
  unviable="${unviable:-0}"
  printf '%s %s %s %s %s\n' "${total}" "${caught}" "${missed}" "${timeout}" "${unviable}"
}

# viable = generated-tested mutants that at least compiled.
mutation_viable() {
  local total="$1" unviable="$2"
  local v=$((total - unviable))
  if ((v < 0)); then v=0; fi
  echo "${v}"
}

# Score percentage with four decimals; timeouts are NOT added to caught, so
# they count as unresolved-missed toward the threshold.
mutation_score() {
  local caught="$1" viable="$2"
  if [[ "${viable}" -gt 0 ]]; then
    awk -v c="${caught}" -v v="${viable}" 'BEGIN { printf "%.4f", c*100/v }'
  else
    echo ""
  fi
}

# PASS when score >= threshold, FAIL otherwise. Float-safe via awk.
mutation_verdict() {
  local score="$1" threshold="$2"
  if awk -v s="${score}" -v t="${threshold}" 'BEGIN { exit !(s >= t) }'; then
    echo "PASS"
  else
    echo "FAIL"
  fi
}

# True when the path is a production Rust file under src/ or crates/.
mutation_is_production_file() {
  local path="$1"
  if [[ "${path}" =~ ^(src/|crates/).*[.]rs$ ]]; then
    return 0
  fi
  return 1
}

# The single CI-parsable summary line. Emitted on stdout and into summary.txt.
mutation_summary_line() {
  local profile="$1" exit_code="$2" total="$3" caught="$4" viable="$5"
  local missed="$6" timeout="$7" unviable="$8" excluded="$9"
  local score="${10}" threshold="${11}" result="${12}"
  if [[ -z "${score}" ]]; then score="n/a"; fi
  printf 'MUTATION_SUMMARY: profile=%s exit=%s total=%s caught=%s viable=%s missed=%s timeout=%s unviable=%s excluded=%s score=%s threshold=%s result=%s\n' \
    "${profile}" "${exit_code}" "${total}" "${caught}" "${viable}" \
    "${missed}" "${timeout}" "${unviable}" "${excluded}" "${score}" "${threshold}" "${result}"
}

# ============================================================================
# Embedded self-test — `scripts/mutation_test.sh --self-test`.
# Lists each command and its expected output. No git, cargo, or filesystem
# side effects: only the pure helpers above are exercised.
# ============================================================================
mutation_self_test() {
  local _st_failures=0
  local got=
  assert_eq() {
    local d="$1" w="$2" g="$3"
    if [[ "${w}" == "${g}" ]]; then
      echo "ok - ${d}"
    else
      echo "not ok - ${d} (want '${w}', got '${g}')" >&2
      _st_failures=$((_st_failures + 1))
    fi
  }
  assert_status() {
    local d="$1" w="$2"
    shift 2
    local r=0
    if "$@" >/dev/null 2>&1; then r=0; else r=1; fi
    if [[ "${w}" == "${r}" ]]; then
      echo "ok - ${d}"
    else
      echo "not ok - ${d} (want status '${w}', got '${r}')" >&2
      _st_failures=$((_st_failures + 1))
    fi
  }

  # classification: parse a full summary
  got="$(mutation_parse_summary '42 mutants tested in 10.2s: 30 caught, 5 missed, 3 timeouts, 4 unviable')"
  assert_eq "parse full summary" "42 30 5 3 4" "${got}"
  # classification: singular forms and absent buckets default to 0
  got="$(mutation_parse_summary '1 mutant tested in 0.5s: 1 caught')"
  assert_eq "parse singular summary" "1 1 0 0 0" "${got}"
  # classification: "timeout" (singular) and word order variants
  got="$(mutation_parse_summary '9 mutants tested in 5.0s: 1 timeout, 1 missed, 7 caught')"
  assert_eq "parse singular timeout" "9 7 1 1 0" "${got}"
  # classification: zero-mutant run
  got="$(mutation_parse_summary '0 mutants tested in 0.1s')"
  assert_eq "parse zero summary" "0 0 0 0 0" "${got}"

  # denominator: unviable mutants are excluded from viable
  got="$(mutation_viable 42 4)"
  assert_eq "viable = total - unviable" "38" "${got}"
  got="$(mutation_viable 2 10)"
  assert_eq "viable clamps at 0" "0" "${got}"

  # REGRESSION: a timeout must NOT count as caught — score drops.
  got="$(mutation_score 30 38)"
  assert_eq "timeout counts as missed toward score (30/38)" "78.9474" "${got}"
  got="$(mutation_score 1 2)"
  assert_eq "timeout halves score instead of passing (1/2)" "50.0000" "${got}"
  got="$(mutation_score 38 38)"
  assert_eq "perfect catch rate" "100.0000" "${got}"
  got="$(mutation_score 5 0)"
  assert_eq "no viable mutants yields empty score" "" "${got}"

  # verdicts, including equality at the threshold
  got="$(mutation_verdict 85.0000 85)"
  assert_eq "verdict pass at threshold" "PASS" "${got}"
  got="$(mutation_verdict 84.9999 85)"
  assert_eq "verdict below threshold" "FAIL" "${got}"
  got="$(mutation_verdict 90 75)"
  assert_eq "verdict above threshold" "PASS" "${got}"

  # in-diff scope: only src/**/*.rs and crates/**/*.rs count
  assert_status "accepts src/foo.rs" 0 mutation_is_production_file "src/foo.rs"
  assert_status "accepts crates/pkg/src/lib.rs" 0 mutation_is_production_file "crates/csm-core/src/lib.rs"
  assert_status "rejects tests/foo.rs" 1 mutation_is_production_file "tests/foo.rs"
  assert_status "rejects root main.rs" 1 mutation_is_production_file "main.rs"
  assert_status "rejects docs" 1 mutation_is_production_file "docs/api.md"
  assert_status "rejects non-rs under src/" 1 mutation_is_production_file "src/main.c"

  # machine inventory line (deterministic key=value) — regression check
  got="$(mutation_summary_line fast 2 42 30 38 5 3 4 15 78.9474 85 FAIL)"
  assert_eq "summary line keys/order" \
    "MUTATION_SUMMARY: profile=fast exit=2 total=42 caught=30 viable=38 missed=5 timeout=3 unviable=4 excluded=15 score=78.9474 threshold=85 result=FAIL" \
    "${got}"
  got="$(mutation_summary_line fast 0 0 0 0 0 0 0 0 "" 85 EMPTY)"
  assert_eq "summary line empty run" \
    "MUTATION_SUMMARY: profile=fast exit=0 total=0 caught=0 viable=0 missed=0 timeout=0 unviable=0 excluded=0 score=n/a threshold=85 result=EMPTY" \
    "${got}"

  if [[ "${_st_failures}" -eq 0 ]]; then
    echo "self-test PASS"
    return 0
  fi
  echo "self-test FAILED (${_st_failures} assertion(s))" >&2
  return 1
}

# ============================================================================
# CLI parsing
# ============================================================================
CI_MODE=false
THRESHOLD="${MUTATION_THRESHOLD:-85}"
SELF_TEST=false
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
    --self-test)
      SELF_TEST=true
      shift
      ;;
    *)
      POSITIONAL+=("$1")
      shift
      ;;
  esac
done

set -- "${POSITIONAL[@]}"

if [[ "${SELF_TEST}" == "true" ]]; then
  if mutation_self_test; then
    exit 0
  fi
  exit 1
fi

if ! [[ "${THRESHOLD}" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  echo "error: --threshold value '${THRESHOLD}' is not a number" >&2
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  exit 2
fi

PROFILE="${1:-fast}"
shift || true

# Parallelism: default 4 on CI (runner has 4 vCPUs), 1 locally
JOBS="${MUTANTS_JOBS:-1}"

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

# Deterministic (non-timestamped) machine-readable artifact directory.
ARTIFACT_DIR="target/mutation-artifacts/${PROFILE}"
mkdir -p "${ARTIFACT_DIR}"

HELP_TEXT="$(cargo mutants --help 2>/dev/null || true)"
FAST_ARGS=()
PRODUCTION_FILES=()

# Shared exclusion filters — applied to the real run; the candidate
# enumeration below intentionally does NOT use them so the "excluded" count
# reflects everything the filters (and the diff scope) removed.
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
  --exclude-re "replace > with >= in <impl Reranker for MmrReranker>::rerank"
)

if [[ "${PROFILE}" == "fast" ]]; then
  if grep -q -- '--in-diff' <<<"${HELP_TEXT}"; then
    DIFF_TARGET="${DIFF_TARGET:-origin/main}"
    DIFF_FILE="$(mktemp)"
    trap 'rm -f "${DIFF_FILE}"' EXIT

    # FAIL-CLOSED scope scan: compute the diff ONCE; if it cannot be computed
    # for the requested base, refuse to guess. Transparently reuses the diff
    # both for --in-diff and for the changed-production-file check.
    if ! git diff "${DIFF_TARGET}" > "${DIFF_FILE}" 2>/dev/null; then
      echo "error: could not compute git diff against '${DIFF_TARGET}'; the --in-diff run cannot be scoped" >&2
      echo "hint: fetch the base ref, or set DIFF_TARGET (e.g. DIFF_TARGET=main) to an existing ref" >&2
      exit 1
    fi
    if [[ ! -s "${DIFF_FILE}" ]]; then
      if [[ "${CI_MODE}" == "true" ]]; then
        echo "error: diff against '${DIFF_TARGET}' is empty — there are no changes to test; failing closed instead of a silent empty run" >&2
        echo "hint: run 'scripts/mutation_test.sh full' to test the whole tree without diff scoping" >&2
        exit 1
      fi
      # Local tooling convenience: an unchanged tree means there is nothing to
      # scope, so fall back to the full target set (loud, not empty).
      echo "warning: no diff against ${DIFF_TARGET}; running full target set" >&2
    else
      mapfile -t PRODUCTION_FILES < <(git diff --name-only "${DIFF_TARGET}" 2>/dev/null | while IFS= read -r p; do
        if mutation_is_production_file "${p}"; then
          printf '%s\n' "${p}"
        fi
      done)
      if [[ "${#PRODUCTION_FILES[@]}" -eq 0 ]]; then
        echo "error: diff against '${DIFF_TARGET}' changes no production files (src/**/*.rs or crates/**/*.rs); refusing a silent empty mutation run" >&2
        echo "hint: push production-file changes first, or run 'scripts/mutation_test.sh full' to test the whole tree" >&2
        exit 1
      fi
      FAST_ARGS+=(--in-diff "${DIFF_FILE}")
      # Record the scoped files for the report and the artifacts.
      printf '%s\n' "${PRODUCTION_FILES[@]}" > "${ARTIFACT_DIR}/in-diff-production-files.txt"
    fi
  else
    echo "warning: --in-diff is unsupported by installed cargo-mutants; running full target set" >&2
  fi
  # CI mode: reuse target/ cache (safe in disposable checkout) + deterministic order
  if [[ "${CI_MODE}" == "true" ]]; then
    # --in-place is incompatible with parallel jobs (-j)
    if [[ "$JOBS" -eq 1 ]]; then
      FAST_ARGS+=(--in-place)
    fi
    FAST_ARGS+=(--no-shuffle)
  fi
elif [[ "${PROFILE}" != "full" ]]; then
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  echo "       scripts/mutation_test.sh --self-test" >&2
  exit 2
fi

# Count candidate mutants WITHOUT any filters: generated-but-not-tested,
# i.e. the "excluded" inventory number. --list parses sources only; it does
# not build or run tests (no target/ writes, no baseline run).
CANDIDATES=""
CANDIDATE_FILE="$(mktemp)"
if RUSTFLAGS="" cargo mutants --list > "${CANDIDATE_FILE}" 2>/dev/null && [[ -s "${CANDIDATE_FILE}" ]]; then
  CANDIDATES="$(grep -c . "${CANDIDATE_FILE}" || true)"
else
  echo "warning: could not enumerate candidate mutants; 'excluded' will be reported as 0" >&2
fi
rm -f "${CANDIDATE_FILE}"

set -o pipefail
set +e  # cargo-mutants exits 2 when any mutant is missed; we evaluate the score below
MUTANTS_ARGS=("${FAST_ARGS[@]}")
if [[ "$JOBS" -gt 1 ]]; then
  MUTANTS_ARGS+=(-j "$JOBS")
fi

RUSTFLAGS="" cargo mutants "${MUTANTS_ARGS[@]}" \
  --build-timeout 180 \
  --minimum-test-timeout 30 \
  "${EXCLUDE_ARGS[@]}" \
  -o "${ARTIFACT_DIR}" \
  "$@" 2>&1 | tee "${LOG_FILE}"
RESULT="${PIPESTATUS[0]}"
set +o pipefail
set -e  # re-enable errexit for the rest of the script

# ============================================================================
# Classification (always computed: feeds both the report and the artifacts)
# ============================================================================
TOTAL=""
CAUGHT=0
MISSED=0
TIMEOUT=0
UNVIABLE=0
VIABLE=""
SCORE=""
RESULT_CODE=""

SUMMARY_LINE="$(grep -oE "[0-9]+ mutant[s]? tested in .*" "${LOG_FILE}" | tail -1 || true)"
if [[ -n "${SUMMARY_LINE}" ]]; then
  read -r TOTAL CAUGHT MISSED TIMEOUT UNVIABLE <<<"$(mutation_parse_summary "${SUMMARY_LINE}")"
fi

if [[ -z "${TOTAL}" ]]; then
  # No mutant summary at all: either cargo-mutants refused the scope or the
  # run failed before producing results.
  if grep -qE 'No mutants to filter|Diff changes no Rust|Diff file is empty' "${LOG_FILE}" 2>/dev/null; then
    RESULT_CODE="EMPTY"
  else
    RESULT_CODE="PARSE_ERROR"
  fi
else
  VIABLE="$(mutation_viable "${TOTAL}" "${UNVIABLE}")"
  if [[ "${TOTAL}" -eq 0 || "${VIABLE}" -eq 0 ]]; then
    RESULT_CODE="EMPTY"
    SCORE=""
  else
    SCORE="$(mutation_score "${CAUGHT}" "${VIABLE}")"
    if [[ -z "${SCORE}" ]]; then
      # Fallback for older cargo-mutants that only print a bare percentage.
      SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
    fi
    if [[ -n "${SCORE}" ]]; then
      RESULT_CODE="$(mutation_verdict "${SCORE}" "${THRESHOLD}")"
    else
      RESULT_CODE="PARSE_ERROR"
    fi
  fi
fi

# Excluded = generated candidates that never got tested (filtered out and/or
# out of the --in-diff scope).
EXCLUDED=0
if [[ -n "${CANDIDATES}" && -n "${TOTAL}" ]] && (( CANDIDATES > TOTAL )); then
  EXCLUDED=$((CANDIDATES - TOTAL))
fi

# Proven-equivalent mutants: the documented set lives in
# scripts/mutation-equivalent.txt; a mutant counts as proven-equivalent only
# when it is both documented there AND still survived in this run (i.e. it is
# really the same mutant, listed in missed.txt). This is the ONLY source of
# "equivalent" — anything else is genuinely missed.
EQUIVALENT_DOC="${SCRIPT_DIR}/mutation-equivalent.txt"
EQUIVALENT=0
if [[ -f "${EQUIVALENT_DOC}" ]]; then
  while IFS= read -r line; do
    line="${line%%#*}"
    line="$(printf '%s' "${line}" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
    [[ -z "${line}" ]] && continue
    if grep -Fqx -- "${line}" "${ARTIFACT_DIR}/missed.txt" 2>/dev/null; then
      EQUIVALENT=$((EQUIVALENT + 1))
    fi
  done < "${EQUIVALENT_DOC}"
fi

MISSED_REPORTED=$((MISSED - EQUIVALENT))
if ((MISSED_REPORTED < 0)); then
  MISSED_REPORTED=0
fi

if [[ "${TIMEOUT}" -gt 0 ]]; then
  echo "warning: ${TIMEOUT} mutant(s) timed out and are classified UNRESOLVED (counted toward the missed bucket / score)" >&2
fi
if [[ "${UNVIABLE}" -gt 0 ]]; then
  echo "note: ${UNVIABLE} mutant(s) were unviable (did not compile) and are excluded from the score denominator" >&2
fi
if [[ "${RESULT_CODE}" == "EMPTY" ]]; then
  echo "warning: no viable mutants were tested (total=${TOTAL:-0}, unviable=${UNVIABLE}); the run carries no coverage signal" >&2
fi

# ============================================================================
# Inventory artifacts (deterministic dir, refreshed per profile)
# ============================================================================
# Copy the per-category mutant name lists out of cargo-mutants' own
# mutants.out (created inside our artifact dir via -o) and into the
# top-level artifact directory.
for kind in caught missed timeout unviable; do
  if [[ -f "${ARTIFACT_DIR}/mutants.out/${kind}.txt" ]]; then
    cp "${ARTIFACT_DIR}/mutants.out/${kind}.txt" "${ARTIFACT_DIR}/${kind}.txt"
  else
    : > "${ARTIFACT_DIR}/${kind}.txt"
  fi
done
# The documented proven-equivalent set (empty-comment seed file by default).
if [[ -f "${EQUIVALENT_DOC}" ]]; then
  cp "${EQUIVALENT_DOC}" "${ARTIFACT_DIR}/equivalent.txt"
else
  : > "${ARTIFACT_DIR}/equivalent.txt"
fi
if [[ -n "${CANDIDATES}" ]]; then
  printf '%s\n' "${CANDIDATES}" > "${ARTIFACT_DIR}/candidate-count.txt"
fi

SUMMARY_OUTPUT="$(mutation_summary_line \
  "${PROFILE}" "${RESULT}" "${TOTAL:-0}" "${CAUGHT}" "${VIABLE:-0}" \
  "${MISSED_REPORTED}" "${TIMEOUT}" "${UNVIABLE}" "${EXCLUDED}" \
  "${SCORE}" "${THRESHOLD}" "${RESULT_CODE}")"

printf '%s\n' \
  "# mutation inventory — ${PROFILE} — ${TIMESTAMP}" \
  "profile=${PROFILE}" \
  "timestamp=${TIMESTAMP}" \
  "exit=${RESULT}" \
  "total=${TOTAL:-0}" \
  "caught=${CAUGHT}" \
  "missed=${MISSED_REPORTED}" \
  "documented_equivalent=${EQUIVALENT}" \
  "timeout=${TIMEOUT}          # UNRESOLVED: counted toward missed" \
  "unviable=${UNVIABLE}        # excluded from the viable denominator" \
  "viable=${VIABLE:-0}" \
  "excluded=${EXCLUDED}        # generated but filtered out / out of diff scope" \
  "candidate_count=${CANDIDATES:-0}" \
  "score=${SCORE:-n/a}" \
  "threshold=${THRESHOLD}" \
  "result=${RESULT_CODE}" \
  "" \
  "${SUMMARY_OUTPUT}" \
  > "${ARTIFACT_DIR}/summary.txt"

echo "${SUMMARY_OUTPUT}"
echo "mutation-inventory: profile=${PROFILE} total=${TOTAL:-0} caught=${CAUGHT} missed=${MISSED_REPORTED} timeout=${TIMEOUT} unviable=${UNVIABLE} excluded=${EXCLUDED} viable=${VIABLE:-0} equivalent=${EQUIVALENT} artifacts=${ARTIFACT_DIR}/"

# ============================================================================
# Report
# ============================================================================
{
  echo "# Mutation Test Report"
  echo
  echo "- Timestamp (UTC): ${TIMESTAMP}"
  echo "- Profile: ${PROFILE}"
  echo "- Exit code: ${RESULT}"
  echo "- Log: \`${LOG_FILE}\`"
  echo "- Command: \`cargo mutants ${FAST_ARGS[*]} $*\`"
  echo
  echo "## Inventory"
  echo
  echo "- Total: ${TOTAL:-n/a}"
  echo "- Caught: ${CAUGHT}"
  echo "- Missed: ${MISSED_REPORTED} (${EQUIVALENT} documented proven-equivalent)"
  echo "- Timeout (UNRESOLVED, counted as missed): ${TIMEOUT}"
  echo "- Unviable (excluded from denominator): ${UNVIABLE}"
  echo "- Excluded (generated but not tested): ${EXCLUDED}"
  echo "- Viable: ${VIABLE:-n/a}"
  echo -e "- Score: ${SCORE:-n/a}% (threshold ${THRESHOLD}%) => ${RESULT_CODE}"
  echo "- Machine-readable inventory: \`${ARTIFACT_DIR}/\` (see summary.txt)"
  echo "- Equivalence guidance: scripts/mutation-equivalence.md"
  if [[ "${#PRODUCTION_FILES[@]}" -gt 0 ]]; then
    echo "- Production files in scope ($([[ "${#PRODUCTION_FILES[@]}" -gt 0 ]] && echo "${#PRODUCTION_FILES[@]}" || echo 0)):"
    printf '%s\n' "${PRODUCTION_FILES[@]}" | sed 's/^/    /'
  fi
  echo
  echo "## Tail"
  echo
  echo '```text'
  tail -n 40 "${LOG_FILE}"
  echo '```'
} >"${REPORT_FILE}"

echo "wrote ${REPORT_FILE}"

# ============================================================================
# Verdict / exit
# ============================================================================
if [[ "${CI_MODE}" == "true" ]]; then
  case "${RESULT_CODE}" in
    PASS)
      echo "mutation score ${SCORE}% >= ${THRESHOLD}%, CI check passed"
      exit 0
      ;;
    FAIL)
      echo "mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
      exit 1
      ;;
    EMPTY)
      echo "error: mutation run produced no viable mutants (total=${TOTAL:-0}, unviable=${UNVIABLE}); failing closed instead of reporting a silent pass" >&2
      echo "hint: if the diff touched only excluded or unviable code, extend the exclusion list or check the cargo-mutants log ${LOG_FILE}" >&2
      exit 1
      ;;
    PARSE_ERROR)
      echo "error: could not parse mutation score from ${LOG_FILE} (cargo-mutants exit ${RESULT})" >&2
      exit 1
      ;;
    *)
      echo "error: unexpected result state '${RESULT_CODE}'" >&2
      exit 1
      ;;
  esac
fi

# Non-CI: propagate cargo-mutants' own verdict (0 = all caught, 2 = missed,
# 3 = timeouts, …). Inventory/summary above already describe the run.
exit "${RESULT}"