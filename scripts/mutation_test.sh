#!/usr/bin/env bash
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
if [[ "${PROFILE}" == "fast" ]]; then
  if grep -q -- '--in-diff' <<<"${HELP_TEXT}"; then
    DIFF_TARGET="${DIFF_TARGET:-origin/main}"
    DIFF_FILE="$(mktemp)"
    trap 'rm -f "${DIFF_FILE}"' EXIT
    git diff "${DIFF_TARGET}" > "${DIFF_FILE}" 2>/dev/null || true
    if [[ -s "${DIFF_FILE}" ]]; then
      FAST_ARGS+=(--in-diff "${DIFF_FILE}")
    else
      echo "warning: no diff against ${DIFF_TARGET}; running full target set" >&2
    fi
  else
    echo "warning: --in-diff is unsupported by installed cargo-mutants; running full target set" >&2
  fi
  # CI mode: reuse target/ cache (safe in disposable checkout) + deterministic order
  if [[ "${CI_MODE}" == "true" ]]; then
    FAST_ARGS+=(--in-place --no-shuffle)
  fi
elif [[ "${PROFILE}" != "full" ]]; then
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  exit 2
fi

set -o pipefail
RUSTFLAGS="" cargo mutants "${FAST_ARGS[@]}" \
  --build-timeout 600 \
  --exclude-re 'WasmFramework::' \
  --exclude-re 'persistence::Persistence::schema_version' \
  --exclude-re 'persistence::Persistence::load_index' \
  --exclude-re 'persistence::Persistence::list_namespaces' \
  --exclude-re 'HnswIndex::serialize' \
  --exclude-re 'HnswIndex::deserialize' \
  --exclude-re 'OtlpGuard::' \
  --exclude-re 'install_grpc_tracer' \
  --exclude-re 'impl Drop for Guard' \
  --exclude-re 'impl Drop for OtlpGuard' \
  --exclude-re 'Result<Option<Guard>>' \
  --exclude-re 'replace && with' \
  --exclude-re 'delete . in init' \
  --exclude-re 'replace > with >= in FrameworkBuilder::with_max_associations_per_concept' \
  --exclude-re 'persistence_wasm' \
  --exclude-re 'replace > with .* in FrameworkBuilder::build' \
  --exclude-re 'apply_migrations_with_conn' \
  --exclude-re 'ChaoticSemanticFramework::load ' \
  "$@" 2>&1 | tee "${LOG_FILE}"
RESULT="${PIPESTATUS[0]}"
set +o pipefail

{
  echo "# Mutation Test Report"
  echo
  echo "- Timestamp (UTC): ${TIMESTAMP}"
  echo "- Profile: ${PROFILE}"
  echo "- Exit code: ${RESULT}"
  echo "- Log: \`${LOG_FILE}\`"
  echo "- Command: \`cargo mutants ${FAST_ARGS[*]} $*\`"
  echo
  echo "## Tail"
  echo
  echo '```text'
  tail -n 40 "${LOG_FILE}"
  echo '```'
} >"${REPORT_FILE}"

echo "wrote ${REPORT_FILE}"

if [[ "${CI_MODE}" == "true" ]]; then
  # Parse mutation score. Supports both percentage output and "X mutants tested ... Y caught" summary.
  # Unviable mutants (won't compile) are excluded from the denominator.
  SCORE=""
  SUMMARY_LINE="$(grep -oE "[0-9]+ mutant[s]? tested in .* [0-9]+ caught.*" "${LOG_FILE}" || true)"
  if [[ -n "${SUMMARY_LINE}" ]]; then
    TOTAL="$(echo "${SUMMARY_LINE}" | awk '{print $1}')"
    CAUGHT="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ caught' | awk '{print $1}')"
    UNVIABLE="$(echo "${SUMMARY_LINE}" | grep -oE '[0-9]+ unviable' | awk '{print $1}')"
    UNVIABLE="${UNVIABLE:-0}"
    VIABLE=$((TOTAL - UNVIABLE))
    if [[ "${VIABLE}" -gt 0 ]]; then
      SCORE="$(awk -v c="${CAUGHT}" -v v="${VIABLE}" 'BEGIN { printf "%.4f", c*100/v }')"
    else
      SCORE="100"
    fi
  fi
  if [[ -z "${SCORE}" ]]; then
    SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
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
    # Score meets threshold: succeed even if cargo-mutants returned non-zero
    # (it exits 2 whenever any mutant is missed, regardless of the threshold).
    exit 0
  else
    echo "mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
    exit 1
  fi
fi

exit "${RESULT}"
