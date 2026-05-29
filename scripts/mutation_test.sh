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
elif [[ "${PROFILE}" != "full" ]]; then
  echo "usage: scripts/mutation_test.sh [--ci] [--threshold=N] [fast|full] [extra cargo-mutants args...]" >&2
  exit 2
fi

set -o pipefail
RUSTFLAGS="" cargo mutants "${FAST_ARGS[@]}" "$@" 2>&1 | tee "${LOG_FILE}"
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
  SCORE="$(awk '/%/{ gsub(/[^0-9.]/," "); for(i=1;i<=NF;i++) if($i ~ /^[0-9]+\.?[0-9]*$/) s=$i } END { print s+0 }' "${LOG_FILE}")"
  if [[ "${SCORE}" == "0" ]]; then
    echo "error: could not parse mutation score from ${LOG_FILE}" >&2
    exit 1
  fi
  if awk -v s="${SCORE}" -v t="${THRESHOLD}" 'BEGIN { exit !(s >= t) }'; then
    echo "mutation score ${SCORE}% >= ${THRESHOLD}%, CI check passed"
  else
    echo "mutation score ${SCORE}% < ${THRESHOLD}%, CI check failed" >&2
    exit 1
  fi
fi

exit "${RESULT}"
